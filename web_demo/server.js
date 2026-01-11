const express = require("express");
const { exec } = require("child_process");
const path = require("path");
const app = express();

app.use(express.json());

// CONFIG: Path to your Rust project
// We use 'cargo run --quiet --' to run the DB without the compilation logs
const RUST_PROJECT_PATH = path.join(__dirname, "../"); // Assumes web_demo is inside rust_sqlite
const DB_COMMAND = `cd ${RUST_PROJECT_PATH} && cargo run --quiet --release --`;

// Helper to run SQL via Rust
function runSQL(query) {
  return new Promise((resolve, reject) => {
    // Escape quotes slightly for bash safety (very basic)
    const safeQuery = query.replace(/"/g, '\\"');

    exec(`${DB_COMMAND} "${safeQuery}"`, (error, stdout, stderr) => {
      if (error) {
        console.error(`Exec error: ${stderr}`);
        return reject(stderr || error.message);
      }
      resolve(stdout.trim());
    });
  });
}

// 1. Initialize DB (Route)
app.get("/init", async (req, res) => {
  try {
    await runSQL("CREATE TABLE users (name TEXT, age INT)");
    res.send("Database initialized and table 'users' created.");
  } catch (e) {
    res.status(500).send("Error (Table likely exists): " + e);
  }
});

// 2. Create User (POST)
app.post("/users", async (req, res) => {
  const { name, age } = req.body;
  try {
    // Note: Simple string interpolation is vulnerable to SQL Injection!
    // But for a trivial demo, it works.
    const output = await runSQL(`INSERT INTO users VALUES ('${name}', ${age})`);
    res.send(output);
  } catch (e) {
    res.status(500).send(e);
  }
});

// 3. List Users (GET)
app.get("/users", async (req, res) => {
  try {
    const output = await runSQL("SELECT * FROM users");
    // Output is a text table. Let's send it as plain text for easy reading.
    res.set("Content-Type", "text/plain");
    res.send(output);
  } catch (e) {
    res.status(500).send(e);
  }
});

app.listen(3000, () => {
  console.log("Web App running on http://localhost:3000");
  console.log(
    'Make sure you have built the rust project via "cargo build --release" first!'
  );
});
