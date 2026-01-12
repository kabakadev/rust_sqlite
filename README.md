# RustDB: A Custom In-Memory RDBMS Engine

![Rust](https://img.shields.io/badge/Built_With-Rust-orange?style=flat-square&logo=rust)
![Node.js](https://img.shields.io/badge/Client-Node.js-green?style=flat-square&logo=node.js)
![Status](https://img.shields.io/badge/Status-Prototype-blue?style=flat-square)

**RustDB** is a lightweight, relational database management system built from scratch in Rust. It implements a client-server architecture where a high-performance Rust backend manages data in memory, serving requests over HTTP to clients (demonstrated here with a Node.js web app).

This project was built to demystify the internals of database systems—moving beyond "using" databases to actually **building** one.

---

## 🌟 Key Features

Despite being a prototype, RustDB implements core RDBMS concepts found in production systems:

- **Client-Server Architecture:** Runs as a persistent HTTP server (Daemon) using `actix-web`, eliminating startup overhead for blazing-fast queries.
- **In-Memory Performance:** Uses Rust's `BTreeMap` for storage, allowing for microsecond-level data retrieval.
- **Disk Persistence:** Automatically serializes and saves state to JSON on success, ensuring data survives restarts.
- **Relational Logic (JOINS):** Supports `INNER JOIN` operations using a nested-loop algorithm to connect data across tables.
- **Type Safety:** Strictly enforces schema types (`Integer`, `Float`, `Text`, `Bool`) on insert.
- **Constraints:** Supports `UNIQUE` constraints (e.g., ensuring unique email addresses).
- **Concurrency:** Uses Mutex locking to handle thread-safe access to the database from the web server.

---

## 🛠️ Architecture Overview

```
┌─────────────────┐          HTTP POST           ┌──────────────────┐
│   Node.js App   │ ────────────────────────────> │   Rust Server    │
│  (Express API)  │          (SQL Query)          │   (actix-web)    │
└─────────────────┘ <──────────────────────────── └──────────────────┘
                             JSON Response                 │
                                                           │
                                                           ▼
                                                  ┌──────────────────┐
                                                  │   SQL Parser     │
                                                  │  (sqlparser-rs)  │
                                                  └──────────────────┘
                                                           │
                                                           ▼
                                                  ┌──────────────────┐
                                                  │  Query Executor  │
                                                  │  (BTreeMap ops)  │
                                                  └──────────────────┘
                                                           │
                                                           ▼
                                                  ┌──────────────────┐
                                                  │  Disk Persistence│
                                                  │   (mydb.json)    │
                                                  └──────────────────┘
```

The system consists of two distinct layers:

### 1. The Engine (Rust)

- **Parser:** Uses `sqlparser` to convert raw SQL text into an Abstract Syntax Tree (AST).
- **Executor:** Interprets the AST, manipulating in-memory data structures (`BTreeMap`).
- **Storage:** Serializes the memory state to `mydb.json` for persistence.
- **Server:** Listens on port `8080` for incoming SQL queries via HTTP.

### 2. The Client (Node.js)

- A lightweight Express.js web server.
- Instead of connecting via a driver, it sends raw SQL strings to the Rust backend via HTTP POST requests.

---

## 🚀 Getting Started

### Prerequisites

- **Rust:** [Install Rust](https://www.rust-lang.org/tools/install) (Ensure `cargo` is in your path)
- **Node.js:** [Install Node.js](https://nodejs.org/) (for the web client demo)
- **Curl:** (Optional) For testing the API directly

### Installation

1. **Clone the repository:**

   ```bash
   git clone https://github.com/your-username/rustdb.git
   cd rustdb
   ```

2. **Build the Rust Engine:**

   ```bash
   cargo build --release
   ```

3. **Install Node.js Dependencies:**
   ```bash
   cd web_demo
   npm install
   ```

---

## 🎮 Usage Guide

You can run RustDB in two modes: **REPL Mode** (Interactive) or **Server Mode** (API).

### Mode 1: The Interactive REPL

Perfect for testing SQL commands directly in your terminal.

```bash
# From the project root
cargo run
```

_You will see the `rdb >` prompt._

**Try these SQL commands:**

```sql
-- 1. Create a table with constraints
CREATE TABLE users (id INT, email TEXT UNIQUE)

-- 2. Insert data (Type checking is active!)
INSERT INTO users VALUES (1, 'ian@test.com')
INSERT INTO users VALUES (2, 'alice@test.com')

-- 3. Update data
UPDATE users SET email = 'new_email@test.com' WHERE id = 1

-- 4. Delete data
DELETE FROM users WHERE id = 2

-- 5. Query with joins
CREATE TABLE posts (id INT, user_id INT, title TEXT)
INSERT INTO posts VALUES (1, 1, 'My First Post')
SELECT * FROM users JOIN posts ON users.id = posts.user_id

-- 6. Exit (Auto-saves to disk)
exit
```

### Mode 2: The Web Server

Run the database as a background service and connect via the Node.js web app.

**1. Start the Rust Backend:**

```bash
# From project root
cargo run -- server
```

_(Output: "Starting RustDB HTTP Server on port 8080...")_

**2. Start the Node.js Client (New Terminal):**

```bash
# From web_demo/ directory
node server.js
```

_(Output: "Web App running on http://localhost:3000")_

**3. Test the API:**

You can now use `curl` to interact with your full stack!

```bash
# Get all users (Proxies request to Rust)
curl http://localhost:3000/users

# Add a user (Sends JSON -> Node -> SQL -> Rust)
curl -X POST -H "Content-Type: application/json" \
     -d '{"name": "WebUser", "age": 99}' \
     http://localhost:3000/users
```

---

## 📋 Supported SQL Operations

### Data Definition Language (DDL)

- `CREATE TABLE table_name (col1 TYPE, col2 TYPE UNIQUE)`
  - Supported types: `INT`, `FLOAT`, `TEXT`, `BOOL`
  - Constraints: `UNIQUE`

### Data Manipulation Language (DML)

- `INSERT INTO table_name VALUES (value1, value2, ...)`
- `SELECT * FROM table_name`
- `SELECT * FROM table1 JOIN table2 ON table1.col = table2.col`
- `UPDATE table_name SET col = value WHERE id = value`
- `DELETE FROM table_name WHERE id = value`

---

## ⚠️ Current Limitations (Work in Progress)

This project is an educational prototype and serves as a foundation for advanced systems programming. Current limitations include:

- **Scalability:** The entire dataset must fit in RAM. It does not yet support paging to disk for massive datasets.
- **Durability:** Data is saved to disk only after a successful operation. A power failure _during_ a write could theoretically corrupt the JSON file (No Write-Ahead Log/ACID transactions yet).
- **Query Support:** Currently supports `SELECT`, `INSERT`, `UPDATE`, `DELETE`, and `INNER JOIN`. Complex features like `GROUP BY`, `ORDER BY`, or nested subqueries are on the roadmap.
- **SQL Dialect:** Strict syntax requirements (e.g., currently `DELETE` only supports `WHERE id = X`).
- **Concurrency:** Last-write-wins model. Concurrent modifications can result in data loss without proper file locking.
- **WHERE Clauses:** Limited support - primarily works with ID-based conditions.

---

## 🏗️ Technical Implementation Details

### Data Structures

```rust
// Core data structures
Database {
    tables: HashMap<String, Table>
}

Table {
    name: String,
    columns: Vec<(String, String)>,      // Preserves order
    unique_constraints: HashSet<String>,  // Column names with UNIQUE
    data: BTreeMap<u32, Row>,            // Auto-sorted by ID
    last_id: u32
}

Row {
    id: u32,
    data: BTreeMap<String, Value>
}

Value {
    Integer(i64) | Float(f64) | Text(String) | Bool(bool) | Null
}
```

### Key Rust Concepts Demonstrated

1. **Pattern Matching:** Destructuring complex SQL AST nodes
2. **Ownership & Borrowing:** Safe memory management without garbage collection
3. **Error Handling:** Using `Result<T, E>` for robust error propagation
4. **Trait Derivation:** Automatic serialization with `serde`
5. **Collections:** Efficient use of `HashMap`, `BTreeMap`, and `Vec`
6. **Concurrency:** Thread-safe server using `Arc<Mutex<Database>>`

### Join Algorithm

RustDB implements a **nested-loop join** algorithm:

```rust
for left_row in left_table.rows {
    for right_row in right_table.rows {
        if left_row[left_col] == right_row[right_col] {
            merged_row = merge(left_row, right_row)
            results.push(merged_row)
        }
    }
}
```

Time complexity: O(n × m) where n and m are row counts.

---

## 🧪 Testing

### Run the Test Suite

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests
```

### Manual Testing Checklist

- [ ] CREATE TABLE with type enforcement
- [ ] INSERT with type validation
- [ ] UNIQUE constraint enforcement
- [ ] SELECT retrieval
- [ ] INNER JOIN across tables
- [ ] UPDATE operations
- [ ] DELETE operations
- [ ] Data persistence across restarts
- [ ] Server mode with HTTP requests

---

## 🤝 Attribution & Credit

This project was built with an **AI-Assisted Development** workflow:

- **Architectural Design & Debugging:** Collaboration with **Google Gemini** and **Anthropic Claude**
- **Core Implementation:** The logic, testing, and integration of the Rust/Node.js stack were synthesized and verified by the author
- **Learning Journey:** Built over multiple iterations to deeply understand database internals and Rust systems programming

_Special thanks to the open-source Rust community for the `sqlparser`, `serde`, and `actix-web` crates._

---

## 🔮 Roadmap

Future increments planned for this engine:

- [ ] Implementation of `WHERE` clauses for non-ID columns
- [ ] Binary file format (replacing JSON) for faster persistence
- [ ] Aggregation functions (`COUNT`, `SUM`, `AVG`)
- [ ] `GROUP BY` and `ORDER BY` support
- [ ] Basic authentication for the HTTP server
- [ ] Write-Ahead Logging (WAL) for ACID transactions
- [ ] B-Tree on-disk storage for datasets larger than RAM
- [ ] Query optimization and execution planning
- [ ] Connection pooling for the HTTP server

---

## 📝 What Was Accomplished

This project successfully demonstrates:

✅ **Systems Programming:** Manual memory management and file I/O in Rust  
✅ **Data Structures:** Using `BTreeMap` and `Vec` to model complex relationships  
✅ **Parsing:** Tokenizing and interpreting raw SQL text  
✅ **Networking:** Implementing a thread-safe HTTP server with Mutex locking  
✅ **Interoperability:** Connecting Node.js and Rust  
✅ **Type Safety:** Compile-time guarantees and runtime type enforcement  
✅ **Relational Algebra:** Implementing JOIN operations from first principles

### Example Session

```sql
rdb > CREATE TABLE users (id INT, email TEXT UNIQUE)
Success: Table 'users' created

rdb > INSERT INTO users VALUES (1, 'ian@test.com')
Success: Inserted 1 rows

rdb > INSERT INTO users VALUES (2, 'ian@test.com')
Error: UNIQUE constraint violated on column 'email'

rdb > CREATE TABLE posts (id INT, user_id INT, title TEXT)
Success: Table 'posts' created

rdb > INSERT INTO posts VALUES (1, 1, 'My First Post')
Success: Inserted 1 rows

rdb > SELECT * FROM users JOIN posts ON users.id = posts.user_id
ID | id | email | id | user_id | title
--------------------
1  | 1 | ian@test.com | 1 | 1 | My First Post
Success: Returned 1 rows

rdb > UPDATE users SET email = 'new@test.com' WHERE id = 1
Success: Updated 1 rows

rdb > DELETE FROM users WHERE id = 2
Success: Deleted 1 rows
```

---

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

## 🙏 Acknowledgments

Built as an educational exploration of database internals and Rust systems programming. This project represents a journey from "using" databases to truly understanding how they work under the hood.

**Status:** Portfolio-ready prototype demonstrating production RDBMS concepts in ~500 lines of Rust.
