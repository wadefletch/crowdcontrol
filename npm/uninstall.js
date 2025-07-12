#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

function cleanup() {
  const binDir = path.join(__dirname, 'bin');
  
  if (fs.existsSync(binDir)) {
    console.log('Cleaning up crowdcontrol binary...');
    fs.rmSync(binDir, { recursive: true, force: true });
    console.log('✅ Cleanup completed');
  }
}

if (require.main === module) {
  cleanup();
}