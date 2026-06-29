#!/usr/bin/env node
// ESM entry point for schemalint-zod.
// Imports from the compiled dist/ so this bin works in a published install
// without requiring tsx. (In the monorepo dev path, tsx also runs compiled JS fine.)
import { main } from '../dist/server.js';
main();
