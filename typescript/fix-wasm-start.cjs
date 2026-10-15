#!/usr/bin/env node
/**
 * Post-build script to fix the __wbindgen_start call in the generated WASM wrapper.
 * The function may not exist depending on build configuration, causing runtime errors.
 */

const fs = require('fs');
const path = require('path');

const wasmFile = path.join(__dirname, 'pkg', 'cooklang_wasm.js');

// wasm-pack writes pkg/.gitignore containing `*`. npm >= 11 honours nested
// ignore files even for paths listed in package.json "files", which silently
// drops pkg/ from the published tarball (this shipped a broken 0.19.0).
fs.rmSync(path.join(__dirname, 'pkg', '.gitignore'), { force: true });

try {
    let content = fs.readFileSync(wasmFile, 'utf8');

    // Replace unconditional __wbindgen_start call with conditional
    const before = 'wasm.__wbindgen_start();';
    const after = `// Only call __wbindgen_start if it exists (it may not be exported for all build configurations)
if (typeof wasm.__wbindgen_start === 'function') {
    wasm.__wbindgen_start();
}`;

    if (content.includes(before)) {
        content = content.replace(before, after);
        fs.writeFileSync(wasmFile, content, 'utf8');
        console.log('✅ Fixed __wbindgen_start call in pkg/cooklang_wasm.js');
    } else {
        console.log('ℹ️  No __wbindgen_start call found (already patched or not needed)');
    }
} catch (error) {
    console.error('❌ Error patching pkg/cooklang_wasm.js:', error.message);
    process.exit(1);
}
