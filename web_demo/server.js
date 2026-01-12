const express = require("express");

const app = express();

app.use(express.json());

async function runSQL(query) {
  try {
    const response = await fetch("http://127.0.0.1:8080/query", {
      method: "POST",
      body: query,
    });
    const text = await response.text();
    if (!response.ok) throw new Error(text);
    return text;
  } catch (e) {
    throw e;
  }
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
    const output = await runSQL(`INSERT INTO users VALUES ('${name}', ${age})`);
    res.send(output);
  } catch (e) {
    console.error("Server Error:", e); // Log to terminal
    res.status(500).send(e.toString()); // Send text to curl, NOT empty JSON
  }
});
``;

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
