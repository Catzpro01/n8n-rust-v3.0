// SPDX-License-Identifier: AGPL-3.0-or-later
// Minimal static server for running /bench.html against a local dist build.
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.cwd(), process.argv[2] || "dist");
const port = Number(process.argv[3] || 8787);
const types = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".cwbt": "application/vnd.canopy.topology+v1",
  ".png": "image/png",
};
http
  .createServer((req, res) => {
    let p = decodeURIComponent((req.url || "/").split("?")[0]);
    if (p === "/") p = "/bench.html";
    const file = path.normalize(path.join(root, p));
    if (!file.startsWith(root)) {
      res.writeHead(403);
      res.end("outside root");
      return;
    }
    fs.readFile(file, (err, data) => {
      if (err) {
        res.writeHead(404);
        res.end(err.message);
        return;
      }
      const ext = path.extname(file);
      res.writeHead(200, { "content-type": types[ext] || "application/octet-stream" });
      res.end(data);
    });
  })
  .listen(port, "127.0.0.1", () => {
    console.log(JSON.stringify({ event: "listening", port, root }));
  });
