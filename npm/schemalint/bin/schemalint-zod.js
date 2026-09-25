#!/usr/bin/env node
// Enable V8's compile cache before anything heavy is compiled so warm runs
// reuse bytecode for tsx, dist/*.js, and the dynamically-imported
// `typescript` package. Guarded: Node < 22 lacks module.enableCompileCache
// and engines allow >=18. Dynamic imports below are load-bearing: static ESM
// dependencies are evaluated before this entry module's body, so they would
// compile before enableCompileCache() runs.
import module from 'node:module';
module.enableCompileCache?.();

// Register the package-local TypeScript loader before the sidecar dynamically
// imports user modules.
await import('tsx');

const { main } = await import('../dist/server.js');
main();
