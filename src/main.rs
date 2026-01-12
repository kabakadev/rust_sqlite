use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::File; 
use std::io::BufReader;
use std::path::Path;

use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use sqlparser::ast::{Statement, DataType, SetExpr, Values, ColumnOption}; 

// 1. Data Types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Null,
}

// 2. The Row
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Row {
    pub id: u32,
    pub data: BTreeMap<String, Value>,
}

// 3. The Table
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<(String, String)>,
    pub unique_columns: Vec<String>,
    pub data: BTreeMap<u32, Row>,
    pub last_id: u32,
}

impl Table {
    pub fn new(name: String) -> Self {
        Table {
            name,
           columns: Vec::new(),
           unique_columns: Vec::new(),
            data: BTreeMap::new(),
            last_id: 0,
        }
    }
}

// 4. The Database Manager
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

    pub fn save_to_disk(&self) -> Result<(), Box<dyn Error>> {
        let file = File::create("mydb.json")?;
        serde_json::to_writer_pretty(file, &self)?;
        Ok(())
    }

    pub fn load_from_disk() -> Result<Self, Box<dyn Error>> {
        if Path::new("mydb.json").exists() {
            let file = File::open("mydb.json")?;
            let reader = BufReader::new(file);
            let db: Database = serde_json::from_reader(reader)?;
            return Ok(db);
        }
        Ok(Database::new())
    }
}

// 5. The Main Entry Point
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut db = Database::load_from_disk().unwrap_or_else(|_| Database::new());

    // MODE 1: One-Shot Command (for Web App)
    if args.len() > 1 {
        let input = &args[1];
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, input);
        
        match ast {
            Ok(statements) => {
                if !statements.is_empty() {
                    match process_command(&mut db, &statements[0]) {
                        Ok(msg) => {
                            println!("{}", msg);
                            db.save_to_disk()?;
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
            }
            Err(e) => eprintln!("SQL Syntax Error: {:?}", e),
        }
        return Ok(());
    }

    // MODE 2: Interactive REPL
    println!("Welcome to RustDB!");
    println!("Loading database...");
    
    let mut rl = rustyline::DefaultEditor::new()?;
    loop {
        let readline = rl.readline("rdb > ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.eq_ignore_ascii_case("exit") {
                    println!("Saving and exiting...");
                    db.save_to_disk()?;
                    break;
                }
                let _ = rl.add_history_entry(input);

                let dialect = GenericDialect {};
                let ast = Parser::parse_sql(&dialect, input);

                match ast {
                    Ok(statements) => {
                        if statements.is_empty() { continue; }
                        match process_command(&mut db, &statements[0]) {
                            Ok(msg) => println!("Success: {}", msg),
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    Err(e) => println!("SQL Syntax Error: {:?}", e),
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

// 6. The Brain (Adjusted for sqlparser 0.39.0)
fn process_command(db: &mut Database, stmt: &Statement) -> Result<String, String> {
    match stmt {
       // CREATE TABLE
        Statement::CreateTable { name, columns, .. } => {
            let table_name = name.to_string();
            if db.tables.contains_key(&table_name) {
                return Err(format!("Table '{}' already exists", table_name));
            }
            let mut table = Table::new(table_name.clone());
            
            for col in columns {
                let col_name = col.name.to_string();
                let col_type = match col.data_type {
                    DataType::Int(_) => "Integer",
                    DataType::Float(_) => "Float",
                    DataType::Text => "Text",
                    DataType::Boolean => "Bool",
                    _ => return Err(format!("Unsupported type: {:?}", col.data_type)),
                };
                table.columns.push((col_name.clone(), col_type.to_string()));

                // CHECK FOR 'UNIQUE' CONSTRAINT
                for option in &col.options {
                    if let ColumnOption::Unique { .. } = &option.option {
                        table.unique_columns.push(col_name.clone());
                    }
                }
            }
            db.tables.insert(table_name.clone(), table);
            Ok(format!("Table '{}' created", table_name))
        }

    // INSERT
        Statement::Insert { table_name, source, .. } => {
            let name = table_name.to_string();
            let table = db.tables.get_mut(&name).ok_or(format!("Table '{}' not found", name))?;
            
            match &*source.body {
                SetExpr::Values(Values { rows, .. }) => {
                    let mut count = 0;
                    for row_expr in rows {
                        let mut row_data = BTreeMap::new();

                        let mut cols_iter = table.columns.iter(); 

                        for expr in row_expr {
                            let (col_name, col_type) = cols_iter.next().ok_or("Too many values for table columns")?;
                            
                            // 1. Convert AST to our Value
                            let value = match expr {
                                sqlparser::ast::Expr::Value(v) => match v {
                                    sqlparser::ast::Value::Number(n, _) => {
                                        if n.contains('.') {
                                            Value::Float(n.parse().unwrap_or(0.0))
                                        } else {
                                            Value::Integer(n.parse().unwrap_or(0))
                                        }
                                    },
                                    sqlparser::ast::Value::SingleQuotedString(s) => Value::Text(s.clone()),
                                    sqlparser::ast::Value::Boolean(b) => Value::Bool(*b),
                                    sqlparser::ast::Value::Null => Value::Null,
                                    _ => return Err("Unsupported value format".to_string()),
                                },
                                _ => return Err("Unsupported expression type".to_string()),
                            };

                            // 2. TYPE CHECK: Verify value matches column schema
                            match (col_type.as_str(), &value) {
                                ("Integer", Value::Integer(_)) => {},
                                ("Float", Value::Float(_)) => {},
                                ("Text", Value::Text(_)) => {},
                                ("Bool", Value::Bool(_)) => {},
                                (_, Value::Null) => {}, // Allow NULL for any type (for now)
                                // Special case: Allow Integer to be promoted to Float if needed? 
                                // For strictness, let's fail.
                                (expected, actual) => {
                                    return Err(format!(
                                        "Type Mismatch! Column '{}' expects {}, but got {:?}", 
                                        col_name, expected, actual
                                    ));
                                }
                            }
                            
                            row_data.insert(col_name.clone(), value);
                        }
                        for unique_col in &table.unique_columns {
                            if let Some(new_val) = row_data.get(unique_col) {
                                // Scan all existing rows
                                for existing_row in table.data.values() {
                                    if let Some(existing_val) = existing_row.data.get(unique_col) {
                                        if existing_val == new_val {
                                            return Err(format!(
                                                "Unique constraint violation: Column '{}' already has value {:?}", 
                                                unique_col, new_val
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        
                        table.last_id += 1;
                        table.data.insert(table.last_id, Row { id: table.last_id, data: row_data });
                        count += 1;
                    }
                    Ok(format!("Inserted {} rows", count))
                }
                _ => Err("Only INSERT VALUES is supported".to_string()),
            }
        }
        

        // SELECT (Fixed for 0.39.0)
        Statement::Query(query) => {
            if let SetExpr::Select(select) = &*query.body {
                let table_name = &select.from[0].relation; 
                let name = table_name.to_string();
                let table = db.tables.get(&name).ok_or(format!("Table '{}' not found", name))?;

                let headers: Vec<&String> = table.columns.iter().map(|(name, _)| name).collect();
                println!("ID | {}", headers.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" | "));
                println!("{}", "-".repeat(20));

                for row in table.data.values() {
                    let mut values = vec![];
                    for col in &headers {
                        let val = row.data.get(*col).unwrap_or(&Value::Null);
                        let v_str = match val {
                            Value::Integer(i) => i.to_string(),
                            Value::Float(f) => f.to_string(),
                            Value::Text(t) => t.clone(),
                            Value::Bool(b) => b.to_string(),
                            Value::Null => "NULL".to_string(),
                        };
                        values.push(v_str);
                    }
                    println!("{}  | {}", row.id, values.join(" | "));
                }
                Ok(format!("Returned {} rows", table.data.len()))
            } else {
                Err("Only SELECT statements supported".to_string())
            }
        }

        _ => Err("SQL command not supported yet".to_string()),
    }
}