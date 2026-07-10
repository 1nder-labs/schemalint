#!/usr/bin/env node
// Register the package-local TypeScript loader before the sidecar dynamically
// imports user modules. A dynamic import is load-bearing here: static ESM
// dependencies are evaluated before this entry module's body.
import 'tsx';

const { main } = await import('../dist/server.js');
main();
