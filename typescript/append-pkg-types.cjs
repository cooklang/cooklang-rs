#!/usr/bin/env node

/**
 * Post-build script to append WASM package type declarations to index.d.ts
 * This makes the internal pkg/* module declarations available to consumers
 */

const fs = require('fs');
const path = require('path');

const indexDtsPath = path.join(__dirname, 'index.d.ts');
const pkgTypesDtsPath = path.join(__dirname, 'pkg-types.d.ts');

// Read the pkg-types.d.ts content
const pkgTypesContent = fs.readFileSync(pkgTypesDtsPath, 'utf-8');

// Read the current index.d.ts
let indexContent = fs.readFileSync(indexDtsPath, 'utf-8');

// Check if the declarations are already appended (to make script idempotent)
if (!indexContent.includes('Type declarations for WASM package internals')) {
    // Append the pkg types to index.d.ts
    indexContent += '\n' + pkgTypesContent;

    // Write back to index.d.ts
    fs.writeFileSync(indexDtsPath, indexContent, 'utf-8');

    console.log('✓ Appended WASM package type declarations to index.d.ts');
} else {
    console.log('✓ WASM package type declarations already present in index.d.ts');
}
