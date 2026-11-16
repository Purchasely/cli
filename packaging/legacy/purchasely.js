#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const root = process.env.PURCHASELY_MANAGED_INSTALL_ROOT;
const releaseDir = path.resolve(__dirname, "../..");
if (root) {
  try {
    const current = fs.readFileSync(path.join(root, "current-version"), "utf8").trim();
    const expected = fs.realpathSync(path.join(root, "releases", current));
    if (fs.realpathSync(releaseDir) === expected) {
      spawnSync("sh", [path.join(releaseDir, "install.sh"), "--relaunch", "--root", fs.realpathSync(root)], {
        stdio: "ignore",
        env: process.env,
      });
    }
  } catch (_) {
    // A missing or changing managed release should still run the bridge binary.
  }
}

const binary = path.join(releaseDir, "bin", "purchasely");
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit", env: process.env });
if (result.error) {
  process.stderr.write(`purchasely: ${result.error.message}\n`);
  process.exit(1);
}
if (result.signal) process.exit(128 + (osSignalNumber(result.signal) || 0));
process.exit(result.status == null ? 1 : result.status);

function osSignalNumber(signal) {
  return { SIGHUP: 1, SIGINT: 2, SIGQUIT: 3, SIGKILL: 9, SIGTERM: 15 }[signal];
}
