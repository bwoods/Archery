use arrow_array::{Float32Array, UInt32Array};
use arrow_schema::{DataType, Field, Schema};
use forsyth::{order_triangles_inplace, order_vertices};
use itertools::Itertools;
use reedline_repl_rs::clap::{ArgAction, ArgMatches, FromArgMatches, Subcommand};
use std::collections::{HashMap, hash_map};
use std::error::Error;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;
use storage::RecordBatch;
use storage::storage::{Entry, File};
use wavefront_obj::obj::{Primitive, VTNIndex, parse};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a table from a Wavefront™ file
    Import {
        #[arg(required = true)]
        table: String,

        #[arg(required = true)]
        path: PathBuf,

        #[arg(long, action(ArgAction::SetTrue))]
        overwrite: bool,
    },
}
pub fn commands(args: ArgMatches, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
    Ok(Command::from_arg_matches(&args)?.run(file)?)
}

impl Command {
    pub fn run(self, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
        match self {
            Command::Import {
                path,
                table,
                overwrite,
            } => import(file, path, table, overwrite),
        }
    }
}

fn import(
    file: &mut File,
    path: PathBuf,
    table: String,
    overwrite: bool,
) -> Result<Option<String>, Box<dyn Error>> {
    let parsed = parse(read_to_string(path)?)?;
    let mut txn = file.txn()?;

    for obj in parsed.objects {
        println!("{}", obj.name);

        // Wavefront™ files store use separate offset for v, vn, vt values.
        // 1) Unify them here

        let mut indices: Vec<u32> = Default::default();
        let mut reverse: HashMap<VTNIndex, u32> = Default::default();

        let mut offset_for = |v| {
            let next = reverse.len() as u32;
            match reverse.entry(v) {
                hash_map::Entry::Occupied(entry) => indices.push(*entry.get()),
                hash_map::Entry::Vacant(entry) => indices.push(*entry.insert(next)),
            };
        };

        for geometry in obj.geometry {
            for shape in geometry.shapes {
                match shape.primitive {
                    Primitive::Triangle(a, b, c) => {
                        offset_for(a);
                        offset_for(b);
                        offset_for(c);
                    }
                    _ => {
                        println!("ignoring {:?}; only triangles are supported", shape);
                    }
                }
            }
        }

        let mut vertices: Vec<VTNIndex> = Vec::default();
        vertices.resize(reverse.len(), Default::default());

        for (v, n) in reverse {
            vertices[n as usize] = v;
        }

        // 2) Reorder the (new) offsets for better locality

        order_triangles_inplace(Default::default(), &mut indices, loom::PROBE_SIZE as u16)
            .map_err(|err| err.to_string())?;

        let (vertices, indices) = order_vertices(&vertices, &indices) //
            .map_err(|err| err.to_string())?;

        let mut x = Float32Array::builder(vertices.len());
        let mut y = Float32Array::builder(vertices.len());
        let mut z = Float32Array::builder(vertices.len());

        let mut i = Float32Array::builder(vertices.len());
        let mut j = Float32Array::builder(vertices.len());
        let mut k = Float32Array::builder(vertices.len());

        let mut s = Float32Array::builder(vertices.len());
        let mut t = Float32Array::builder(vertices.len());

        // 3) Convert them to Arrow Arrays

        for vert in vertices {
            x.append_value(obj.vertices[vert.0].x as f32);
            y.append_value(obj.vertices[vert.0].y as f32);
            z.append_value(obj.vertices[vert.0].z as f32);

            match vert.2 {
                Some(vn) => {
                    i.append_value(obj.normals[vn].x as f32);
                    j.append_value(obj.normals[vn].y as f32);
                    k.append_value(obj.normals[vn].z as f32);
                }
                None => {
                    i.append_null();
                    j.append_null();
                    k.append_null();
                }
            }

            match vert.1 {
                Some(vt) => {
                    s.append_value(obj.tex_vertices[vt].u as f32);
                    t.append_value(obj.tex_vertices[vt].v as f32);
                }
                None => {
                    s.append_null();
                    t.append_null();
                }
            }
        }

        let schema = Schema::new(vec![
            Field::new("x", DataType::Float32, false), // tenths?
            Field::new("y", DataType::Float32, false),
            Field::new("z", DataType::Float32, false),
            Field::new("i", DataType::Float32, true), // spherical?
            Field::new("j", DataType::Float32, true),
            Field::new("k", DataType::Float32, true),
            Field::new("s", DataType::Float32, true), // Uint16
            Field::new("t", DataType::Float32, true),
        ]);

        let batch = arrow_array::RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(x.finish()),
                Arc::new(y.finish()),
                Arc::new(z.finish()),
                Arc::new(i.finish()),
                Arc::new(j.finish()),
                Arc::new(k.finish()),
                Arc::new(s.finish()),
                Arc::new(t.finish()),
            ],
        )
        .map(RecordBatch::from)?;
        // println!("\t{} v. bytes", batch.get_array_memory_size());

        let mut name = table.clone();
        name.push('.');
        name.push_str(&obj.name);
        name.push('.');
        name.push_str("vs");

        match txn.entry(&name)? {
            Entry::Occupied(entry) => {
                if overwrite == false {
                    Err(redb::TableError::TableExists(name))?
                }

                entry.remove_entry()?.insert_entry(batch)?;
            }
            Entry::Vacant(entry) => {
                entry.insert_entry(batch)?;
            }
        };

        let mut a = UInt32Array::builder(indices.len());
        let mut b = UInt32Array::builder(indices.len());
        let mut c = UInt32Array::builder(indices.len());

        for triangle in indices.iter().copied().tuples::<(u32, u32, u32)>() {
            a.append_value(triangle.0);
            b.append_value(triangle.1);
            c.append_value(triangle.2);
        }

        let schema = Schema::new(vec![
            Field::new("a", DataType::UInt32, false),
            Field::new("b", DataType::UInt32, false),
            Field::new("c", DataType::UInt32, false),
        ]);

        let batch = arrow_array::RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(a.finish()),
                Arc::new(b.finish()),
                Arc::new(c.finish()),
            ],
        )
        .map(RecordBatch::from)?;
        // println!("\t{} f. bytes", batch.get_array_memory_size());

        let mut name = table.clone();
        name.push('.');
        name.push_str(&obj.name);
        name.push('.');
        name.push_str("fs");

        match txn.entry(&name)? {
            Entry::Occupied(entry) => {
                if overwrite == false {
                    Err(redb::TableError::TableExists(name))?
                }

                entry.remove_entry()?.insert_entry(batch)?;
            }
            Entry::Vacant(entry) => {
                entry.insert_entry(batch)?;
            }
        };
    }

    txn.commit()?;

    Ok(None)
}
