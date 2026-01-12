const express = require("express");
const app = express();
const cors = require("cors");

app.use(express.json());
app.use(cors());
app.use(express.static("public"));

const DB_URL = "http://127.0.0.1:8080/query";

async function queryDB(sql) {
  try {
    const response = await fetch(DB_URL, { method: "POST", body: sql });
    const text = await response.text();
    if (!response.ok) throw new Error(text);
    return text;
  } catch (e) {
    console.error("DB Error:", e); // Optional: Comment this out to reduce noise
    throw e;
  }
}

// --- NEW: AUTO-INITIALIZATION ---
async function initDB() {
  console.log("Bootstrapping Database...");
  try {
    // Try to create Categories table
    // We ignore errors because if it exists, that's fine!
    await queryDB("CREATE TABLE categories (id INT, name TEXT UNIQUE)").catch(
      () => {}
    );
    console.log("✔ Categories Table Ready");

    // Try to create Products table
    await queryDB(
      "CREATE TABLE products (id INT, name TEXT, price FLOAT, stock INT, category_id INT)"
    ).catch(() => {});
    console.log("✔ Products Table Ready");
  } catch (e) {
    console.error("Bootstrap Failed:", e);
  }
}

function parseOutput(text) {
  const lines = text.trim().split("\n");
  if (lines.length < 2) return [];
  const headers = lines[0].split(" | ").map((h) => h.trim());
  const data = [];
  for (let i = 1; i < lines.length; i++) {
    const values = lines[i].split(" | ").map((v) => {
      let clean = v.trim();
      const match = clean.match(/^[a-zA-Z]+\((.*)\)$/);
      if (match) {
        clean = match[1];
        if (clean.startsWith('"') && clean.endsWith('"'))
          clean = clean.slice(1, -1);
      }
      return clean;
    });
    let row = {};
    headers.forEach((h, index) => (row[h] = values[index]));
    data.push(row);
  }
  return data;
}

// --- ROUTES (Same as before) ---
app.get("/api/inventory", async (req, res) => {
  try {
    const sql =
      "SELECT * FROM products JOIN categories ON products.category_id = categories.id";
    const raw = await queryDB(sql);
    res.json(parseOutput(raw));
  } catch (e) {
    res.status(500).json({ error: e.toString() });
  }
});

app.get("/api/categories", async (req, res) => {
  try {
    const raw = await queryDB("SELECT * FROM categories");
    res.json(parseOutput(raw));
  } catch (e) {
    res.status(500).json({ error: e.toString() });
  }
});

app.post("/api/products", async (req, res) => {
  let { name, price, stock, category_id } = req.body;
  try {
    if (!price.toString().includes(".")) {
      price = `${price}.0`;
    }
    // Use random ID for Primary Key
    const id = Math.floor(Math.random() * 100000);
    await queryDB(
      `INSERT INTO products VALUES (${id}, '${name}', ${price}, ${stock}, ${category_id})`
    );
    res.json({ success: true });
  } catch (e) {
    res.status(500).json({ error: e.toString() });
  }
});

app.post("/api/categories", async (req, res) => {
  const { name } = req.body;
  try {
    const id = Math.floor(Math.random() * 10000);
    await queryDB(`INSERT INTO categories VALUES (${id}, '${name}')`);
    res.json({ success: true });
  } catch (e) {
    res.status(500).json({ error: e.toString() });
  }
});

app.post("/api/sell/:id", async (req, res) => {
  const id = req.params.id;
  try {
    const allProductsRaw = await queryDB(`SELECT * FROM products`);
    const products = parseOutput(allProductsRaw);
    const product = products.find((p) => p.id == id);
    if (!product) return res.status(404).json({ error: "Product not found" });

    let currentStock = parseInt(product.stock);
    if (currentStock <= 0)
      return res.status(400).json({ error: "Out of stock" });

    const newStock = currentStock - 1;
    await queryDB(`UPDATE products SET stock = ${newStock} WHERE id = ${id}`);
    res.json({ success: true, newStock });
  } catch (e) {
    res.status(500).json({ error: e.toString() });
  }
});

app.delete("/api/products/:id", async (req, res) => {
  try {
    await queryDB(`DELETE FROM products WHERE id = ${req.params.id}`);
    res.json({ success: true });
  } catch (e) {
    res.status(500).json({ error: e.toString() });
  }
});

// STARTUP SEQUENCE
app.listen(3000, async () => {
  // 1. Initialize Tables
  await initDB();
  // 2. Start Server
  console.log("RustMart API running on http://localhost:3000");
});
