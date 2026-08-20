#!/usr/bin/env node
'use strict';

const path = require('path');
const { spawnSync } = require('child_process');
const { ensureBinary } = require('./launcher/ensure-binary.cjs');

async function main() {
  const binaryPath = await ensureBinary();
  const result = spawnSync(binaryPath, process.argv.slice(2), {
    stdio: 'inherit',
    env: {
      ...process.env,
      SCHEMALINT_ZOD_HELPER:
        process.env.SCHEMALINT_ZOD_HELPER
        || path.join(__dirname, 'bin', 'schemalint-zod.js'),
    },
  });
  if (result.error) throw result.error;
  process.exit(result.status ?? 1);
}

main().catch((error) => {
  console.error(`schemalint: ${error.message}`);
  process.exit(1);
});
