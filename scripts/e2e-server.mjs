#!/usr/bin/env node
import { spawn } from "node:child_process";
import { createReadStream } from "node:fs";
import { mkdir, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, join, normalize, resolve, sep } from "node:path";

const distDir = resolve(process.env.E2E_DIST_DIR || "/private/tmp/payflow-playwright-dist");
const host = process.env.HOST || "127.0.0.1";
const port = Number(process.env.PORT || 4173);

function run(command, args, options = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      stdio: ["ignore", "pipe", "pipe"],
      ...options,
    });

    child.stdout.on("data", (chunk) => process.stdout.write(chunk));
    child.stderr.on("data", (chunk) => process.stderr.write(chunk));
    child.on("error", rejectPromise);
    child.on("exit", (code) => {
      if (code === 0) {
        resolvePromise();
      } else {
        rejectPromise(new Error(`${command} ${args.join(" ")} exited with ${code}`));
      }
    });
  });
}

function contentType(path) {
  switch (extname(path)) {
    case ".css":
      return "text/css";
    case ".html":
      return "text/html";
    case ".js":
      return "text/javascript";
    case ".wasm":
      return "application/wasm";
    default:
      return "application/octet-stream";
  }
}

async function buildDist() {
  await mkdir(distDir, { recursive: true });
  const env = {
    ...process.env,
    CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR || "/private/tmp/payflow-playwright-target",
  };
  delete env.NO_COLOR;

  await run(
    "trunk",
    ["build", "--dist", distDir, "--color", "never", "--skip-version-check"],
    { env },
  );
}

function createStaticServer() {
  return createServer(async (request, response) => {
    try {
      const url = new URL(request.url || "/", `http://${host}:${port}`);
      const requestedPath = decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname);
      const filePath = normalize(join(distDir, requestedPath));

      if (!filePath.startsWith(`${distDir}${sep}`)) {
        response.writeHead(403);
        response.end("Forbidden");
        return;
      }

      const fileStat = await stat(filePath);
      if (!fileStat.isFile()) {
        response.writeHead(404);
        response.end("Not found");
        return;
      }

      response.writeHead(200, { "content-type": contentType(filePath) });
      createReadStream(filePath).pipe(response);
    } catch {
      response.writeHead(404);
      response.end("Not found");
    }
  });
}

await buildDist();

const server = createStaticServer();
server.listen(port, host, () => {
  console.log(`Playwright app server listening on http://${host}:${port}`);
});

process.on("SIGTERM", () => server.close(() => process.exit(0)));
process.on("SIGINT", () => server.close(() => process.exit(0)));
