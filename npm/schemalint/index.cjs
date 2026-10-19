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
  console.error(
    'schemalint: the native binary is fetched from GitHub on first run when the\n'
      + 'install script was blocked. Allow it and reinstall to skip this step:\n'
      + '  npm:  npm install @1nder-labs/schemalint (scripts run by default)\n'
      + '  pnpm: pnpm add --allow-build=@1nder-labs/schemalint @1nder-labs/schemalint\n'
      + '        (or run `pnpm approve-builds`)\n'
      + '  bun:  bun add --trust @1nder-labs/schemalint'
  );
  process.exit(1);
});
