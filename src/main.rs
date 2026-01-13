use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::{ File};
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex; // NEW: Needed for locking the DB between web requests

// SQL Parser Imports
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{Statement, DataType, SetExpr, Values, ColumnOption, JoinOperator, JoinConstraint, TableFactor, Expr, BinaryOperator};

// --- DATA STRUCTURES (Same as before) ---
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Row {
    pub id: u32,
    pub data: BTreeMap<String, Value>,
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub tables: HashMap<String, Table>,
}

impl Database {
    pub fn new() -> Self {
        Database { tables: HashMap::new() }
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

// --- LOGIC: The Brain ---
// This handles the SQL logic. It returns a String (success message) or String (error).
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
                    DataType::Bool => "Bool",
                    _ => return Err(format!("Unsupported type: {:?}", col.data_type)),
                };
                table.columns.push((col_name.clone(), col_type.to_string()));

                // Unique Constraint Check
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

                            // 2. TYPE CHECK
                            match (col_type.as_str(), &value) {
                                ("Integer", Value::Integer(_)) => {},
                                ("Float", Value::Float(_)) => {},
                                ("Text", Value::Text(_)) => {},
                                ("Bool", Value::Bool(_)) => {},
                                (_, Value::Null) => {}, 
                                (expected, actual) => {
                                    return Err(format!("Type Mismatch! Column '{}' expects {}, but got {:?}", col_name, expected, actual));
                                }
                            }
                            row_data.insert(col_name.clone(), value);
                        }
                        
                        // 3. UNIQUE CHECK
                        for unique_col in &table.unique_columns {
                            if let Some(new_val) = row_data.get(unique_col) {
                                for existing_row in table.data.values() {
                                    if let Some(existing_val) = existing_row.data.get(unique_col) {
                                        if existing_val == new_val {
                                            return Err(format!("Unique constraint violation: Column '{}' already has value {:?}", unique_col, new_val));
                                        }
                                    }
                                }
                            }
                        }

                        let row_id = if let Some(Value::Integer(provided_id)) = row_data.get("id") {
                            *provided_id as u32 // Use user's ID (e.g. 96600)
                        } else {
                            table.last_id + 1 // Auto-increment if no ID provided
                        };
                        if row_id > table.last_id {
                            table.last_id = row_id;
                        }
                       table.data.insert(row_id, Row { id: row_id, data: row_data });
                       count += 1;
                    }
                    Ok(format!("Inserted {} rows", count))
                }
                _ => Err("Only INSERT VALUES is supported".to_string()),
            }
        }

        // SELECT (With JOIN Support)
        Statement::Query(query) => {
            if let SetExpr::Select(select) = &*query.body {
                let left_table_name = match &select.from[0].relation {
                    TableFactor::Table { name, .. } => name.to_string(),
                    _ => return Err("Only simple table names supported".to_string()),
                };
                let left_table = db.tables.get(&left_table_name).ok_or(format!("Table '{}' not found", left_table_name))?;

                if !select.from[0].joins.is_empty() {
                    // --- JOIN LOGIC ---
                    let join = &select.from[0].joins[0]; 
                    let right_table_name = match &join.relation {
                        TableFactor::Table { name, .. } => name.to_string(),
                        _ => return Err("Only simple table joins supported".to_string()),
                    };
                    let right_table = db.tables.get(&right_table_name).ok_or(format!("Table '{}' not found", right_table_name))?;

                    let (left_col_name, right_col_name) = match &join.join_operator {
                        JoinOperator::Inner(JoinConstraint::On(Expr::BinaryOp { left, op: BinaryOperator::Eq, right })) => {
                            fn extract_col(expr: &Expr) -> Option<String> {
                                match expr {
                                    Expr::Identifier(ident) => Some(ident.value.clone()),
                                    Expr::CompoundIdentifier(idents) => Some(idents.last()?.value.clone()),
                                    _ => None
                                }
                            }
                            match (extract_col(left), extract_col(right)) {
                                (Some(l), Some(r)) => (l, r),
                                _ => return Err("Unsupported ON condition".to_string()),
                            }
                        },
                        _ => return Err("Only INNER JOIN ... ON supported".to_string()),
                    };

                    // Headers
                    let mut headers = vec![];
                    for (col, _) in &left_table.columns { headers.push(format!("{}.{}", left_table_name, col)); }
                    for (col, _) in &right_table.columns { headers.push(format!("{}.{}", right_table_name, col)); }
                    
                    let mut output_lines = Vec::new();
                    output_lines.push(headers.join(" | ")); // Header row

                    // Loop
                    for left_row in left_table.data.values() {
                        for right_row in right_table.data.values() {
                            let l_val = left_row.data.get(&left_col_name).unwrap_or(&Value::Null);
                            let r_val = right_row.data.get(&right_col_name).unwrap_or(&Value::Null);

                            if l_val != &Value::Null && l_val == r_val {
                                let mut row_strs = vec![];
                                for (col, _) in &left_table.columns { row_strs.push(format!("{:?}", left_row.data.get(col).unwrap_or(&Value::Null))); }
                                for (col, _) in &right_table.columns { row_strs.push(format!("{:?}", right_row.data.get(col).unwrap_or(&Value::Null))); }
                                output_lines.push(row_strs.join(" | "));
                            }
                        }
                    }
                    Ok(output_lines.join("\n"))

              } else {
                    // --- STANDARD SELECT (No Join) ---
                    
                    // 1. Determine which columns to show
                    let all_columns: Vec<String> = left_table.columns.iter().map(|(n, _)| n.clone()).collect();
                    let mut target_columns = Vec::new();

                    for item in &select.projection {
                        match item {
                            // If "SELECT *", take everything
                            sqlparser::ast::SelectItem::Wildcard(_) => {
                                target_columns = all_columns.clone();
                                break;
                            },
                            // If "SELECT name", take just that column
                            sqlparser::ast::SelectItem::UnnamedExpr(Expr::Identifier(ident)) => {
                                let col_name = ident.value.clone();
                                if all_columns.contains(&col_name) {
                                    target_columns.push(col_name); 
                                } else {
                                     return Err(format!("Column '{}' not found", col_name));
                                }
                            },
                            _ => return Err("Only SELECT * or SELECT col supported".to_string()),
                        }
                    }

                    // 2. Print Headers
                    let header_display: Vec<&str> = target_columns.iter().map(|s| s.as_str()).collect();
                    

                    // 3. Print Rows (Only the requested columns)
                    let mut output_lines = Vec::new();
                    // Note: We reconstruct the output string for the Server response too
                    output_lines.push(format!("ID | {}", header_display.join(" | ")));

                    for row in left_table.data.values() {
                        let mut values = vec![];
                        for col in &target_columns {
                            let val = row.data.get(col).unwrap_or(&Value::Null);
                            let v_str = match val {
                                Value::Integer(i) => i.to_string(),
                                Value::Float(f) => f.to_string(),
                                Value::Text(t) => t.clone(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "NULL".to_string(),
                            };
                            values.push(v_str);
                        }
                        
                        output_lines.push(format!("{}  | {}", row.id, values.join(" | "))); // Save for Server
                    }
                    Ok(output_lines.join("\n"))
                }
            } else {
                Err("Only SELECT statements supported".to_string())
            }
        }

      // DELETE (Fixed for standard 'DELETE FROM table')
        Statement::Delete { from, tables, selection, .. } => {
            // 1. Determine the table name
            // Standard SQL "DELETE FROM table" uses the 'from' field.
            // Non-standard "DELETE table FROM..." uses the 'tables' field.
            let table_name = if !from.is_empty() {
                match &from[0].relation {
                    TableFactor::Table { name, .. } => name.to_string(),
                    _ => return Err("Only simple table names supported".to_string()),
                }
            } else if !tables.is_empty() {
                tables[0].to_string()
            } else {
                return Err("No table specified".to_string());
            };

            let table = db.tables.get_mut(&table_name).ok_or(format!("Table '{}' not found", table_name))?;

            // 2. Extract ID from "WHERE id = X"
            if let Some(Expr::BinaryOp { left, op: BinaryOperator::Eq, right }) = selection {
                let col_name = match &**left { 
                    Expr::Identifier(i) => i.value.clone(), 
                    _ => return Err("Left side must be column name".to_string()) 
                };
                
                if col_name.to_lowercase() != "id" {
                    return Err("For this demo, you can only DELETE by 'id' (e.g. WHERE id = 1)".to_string());
                }

                let id_val = match &**right { 
                    Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse::<u32>().unwrap_or(0), 
                    _ => return Err("ID must be a number".to_string()) 
                };

                if table.data.remove(&id_val).is_some() {
                    Ok(format!("Deleted row with id {}", id_val))
                } else {
                    Err(format!("ID {} not found", id_val))
                }
            } else {
                Err("DELETE must have a WHERE id = X clause".to_string())
            }
        }

        // UPDATE (Simple: UPDATE table SET col = val WHERE id = X)
        Statement::Update { table, assignments, selection, .. } => {
            let name = match &table.relation {
                TableFactor::Table { name, .. } => name.to_string(),
                _ => return Err("Only simple table names supported".to_string()),
            };
            let db_table = db.tables.get_mut(&name).ok_or(format!("Table '{}' not found", name))?;

            // 1. Get ID from WHERE clause
            let id_val = if let Some(Expr::BinaryOp { left, op: BinaryOperator::Eq, right }) = selection {
                 let col = match &**left { Expr::Identifier(i) => i.value.clone(), _ => return Err("Left side must be col".to_string()) };
                 if col.to_lowercase() != "id" { return Err("Only UPDATE WHERE id = ... supported".to_string()); }
                 match &**right { Expr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse::<u32>().unwrap_or(0), _ => return Err("ID must be number".to_string()) }
            } else {
                return Err("Missing WHERE id = clause".to_string());
            };

            // 2. Find Row
            let row = db_table.data.get_mut(&id_val).ok_or(format!("ID {} not found", id_val))?;

            // 3. Apply Assignments
            for assignment in assignments {
                let col_name = assignment.id[0].value.clone();
                let new_val = match &assignment.value {
                    Expr::Value(v) => match v {
                        sqlparser::ast::Value::Number(n, _) => if n.contains('.') { Value::Float(n.parse().unwrap_or(0.0)) } else { Value::Integer(n.parse().unwrap_or(0)) },
                        sqlparser::ast::Value::SingleQuotedString(s) => Value::Text(s.clone()),
                        sqlparser::ast::Value::Boolean(b) => Value::Bool(*b),
                        _ => Value::Null,
                    },
                    _ => return Err("Unsupported value".to_string()),
                };
                
                // (Optional: You should add Type Checking here similar to INSERT)
                row.data.insert(col_name, new_val);
            }
            Ok(format!("Updated row {}", id_val))
        }

        _ => Err("SQL command not supported yet".to_string()),
    }
}

// --- API HANDLER ---
// This allows Node.js to talk to Rust over HTTP
#[post("/query")]
async fn query_endpoint(req_body: String, db: web::Data<Mutex<Database>>) -> impl Responder {
    let input = req_body.trim();
    let dialect = GenericDialect {};
    let ast = Parser::parse_sql(&dialect, input);

    match ast {
        Ok(statements) => {
            if statements.is_empty() { return HttpResponse::BadRequest().body("Empty query"); }
            
            // LOCK THE DB so only one request happens at a time
            let mut db_guard = db.lock().unwrap();
            
            match process_command(&mut *db_guard, &statements[0]) {
                Ok(msg) => {
                    // Auto-save logic
                    let _ = db_guard.save_to_disk();
                    HttpResponse::Ok().body(msg)
                },
                Err(e) => HttpResponse::BadRequest().body(format!("Error: {}", e)),
            }
        }
        Err(e) => HttpResponse::BadRequest().body(format!("SQL Syntax Error: {:?}", e)),
    }
}

// --- MAIN SERVER LOOP ---
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // MODE 1: Web Server (if argument "server" is passed)
    if args.len() > 1 && args[1] == "server" {
        println!("Starting RustDB HTTP Server on port 8080...");
        let db = Database::load_from_disk().unwrap_or_else(|_| Database::new());
        let db_data = web::Data::new(Mutex::new(db));

        return HttpServer::new(move || {
            App::new()
                .app_data(db_data.clone())
                .service(query_endpoint)
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await;
    }

    // MODE 2: Interactive REPL (Default)
    println!("RustDB REPL (Type 'exit' to quit, run with 'server' arg for HTTP mode)");
    let mut db = Database::load_from_disk().unwrap_or_else(|_| Database::new());
    let mut rl = rustyline::DefaultEditor::new().expect("Failed to init readline");

    loop {
        let readline = rl.readline("rdb > ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.eq_ignore_ascii_case("exit") {
                    db.save_to_disk().expect("Failed to save");
                    break;
                }
                let _ = rl.add_history_entry(input);

                let dialect = GenericDialect {};
                let ast = Parser::parse_sql(&dialect, input);
                match ast {
                    Ok(statements) => {
                        if !statements.is_empty() {
                            // Note: In REPL, we don't need the Mutex locking since it's single threaded here
                            match process_command(&mut db, &statements[0]) {
                                Ok(msg) => {
                                    println!("OK: {}", msg);
                                    let _ = db.save_to_disk(); // Auto-save
                                },
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                    }
                    Err(e) => println!("Syntax Error: {:?}", e),
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}