#!/usr/bin/env node
import { startMCPServer } from './index.js';

startMCPServer().catch((err) => {
  console.error('Fatal error starting DiskRipper MCP server:', err);
  process.exit(1);
});
