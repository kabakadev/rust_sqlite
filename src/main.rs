use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs;
use std::path::Path;

// 1. Data Types
// This Enum defines what kind of data our database supports.
// keeping it simple: Integer (i64), Float (f64), Text (String), and Boolean.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Null,
}

// 2. The Row
// A row is just a map of Column Name -> Value.
// We use BTreeMap because it keeps columns sorted, making it easier to read debugging output.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Row {
    pub id: u32, // Every row will have a hidden auto-incrementing ID
    pub data: BTreeMap<String, Value>,
}

// 3. The Table
// A table has a name, a schema (column definitions), and the actual data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, String>, // Maps "age" -> "Integer"
    // We use a BTreeMap for storage. The Key is the ID (u32), the Value is the Row.
    // BTreeMap is efficient for finding items by ID.
    pub data: BTreeMap<u32, Row>,
    pub last_id: u32,
}

impl Table {
    pub fn new(name: String) -> Self {
        Table {
            name,
            columns: HashMap::new(),
            data: BTreeMap::new(),
            last_id: 0,
        }
    }
}

// 4. The Database Manager
// This holds all our tables and handles saving/loading from disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub tables: HashMap<String, Table>,
}

impl Database {
    pub fn new() -> Self {
        Database {
            tables: HashMap::new(),
        }
    }

    // Save state to a file named "mydb.rdb"
    pub fn save_to_disk(&self) -> Result<(), Box<dyn Error>> {
        let encoded: Vec<u8> = bincode::serialize(&self)?;
        fs::write("mydb.rdb", encoded)?;
        Ok(())
    }

    // Load state from disk
    pub fn load_from_disk() -> Result<Self, Box<dyn Error>> {
        if Path::new("mydb.rdb").exists() {
            let data = fs::read("mydb.rdb")?;
            let db: Database = bincode::deserialize(&data)?;
            return Ok(db);
        }
        Ok(Database::new())
    }
}

// 5. The Main Entry Point (The REPL)
fn main() -> Result<(), Box<dyn Error>> {
    println!("Welcome to RustDB!");
    println!("Loading database...");

    // Load existing DB or create a new one
    let mut db = Database::load_from_disk().unwrap_or_else(|_| Database::new());

    // Set up the interactive shell (REPL)
    let mut rl = rustyline::DefaultEditor::new()?;
    
    loop {
        // Read the line
        let readline = rl.readline("rdb > ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.eq_ignore_ascii_case("exit") {
                    println!("Saving and exiting...");
                    db.save_to_disk()?;
                    break;
                }
                
                // Add to history so you can press Up Arrow
                let _ = rl.add_history_entry(input);

                // placeholder for where we will process SQL
                println!("You typed: {}", input); 
            }
            Err(_) => {
                println!("Error reading input, exiting.");
                break;
            }
        }
    }
    Ok(())
}