'use strict';

const { spawnSync } = require('child_process');

function lint(schemaPath, options = {}) {
  const args = ['check', '--format', 'json'];
  if (options.profile) {
    if (Array.isArray(options.profile)) {
      options.profile.forEach((p) => args.push('--profile', p));
    } else {
      args.push('--profile', options.profile);
    }
  }
  args.push(schemaPath);

  const result = spawnSync('schemalint', args, { encoding: 'utf-8' });

  if (result.error && result.error.code === 'ENOENT') {
    throw new Error(
      'schemalint CLI not found on PATH. Install via: npm install -g @schemalint/cli'
    );
  }

  if (result.status !== 0 && result.status !== 1) {
    return { issues: [], error: result.stderr || 'schemalint process error' };
  }

  try {
    return JSON.parse(result.stdout || '{}');
  } catch {
    return { issues: [], error: result.stderr || 'failed to parse output' };
  }
}

module.exports = { lint };
