"""Build the independent JavaScript review for the disposable two-file fixture."""
import json


def build_multifile_syntax_check(force_missing=False):
    paths = ["src/math.js", "src/index.js"]
    if force_missing:
        paths.append("src/intentionally-missing.js")
    return ("const fs = require('node:fs'); const {spawnSync} = require('node:child_process');\n"
        "const paths = " + json.dumps(paths) + ";\n"
        "for (const path of paths) {\n"
        "  if (!fs.existsSync(path)) { console.error('Missing required fixture file: ' + path); process.exit(1); }\n"
        "  const result = spawnSync(process.execPath, ['--check', path], {stdio:'inherit'});\n"
        "  if (result.error) { console.error('Syntax checker failed'); process.exit(1); }\n"
        "  if (result.status !== 0) process.exit(result.status || 1);\n"
        "}\n"
        "console.log('Both required files passed syntax checks');\n")


def build_multifile_review(math_source, index_source):
    # Run the returned script in a separate network-isolated, resource-limited
    # container for model output. This VM is for module wiring, not a sandbox.
    return ("const vm = require('node:vm'); const assert = require('node:assert/strict');\n"
        "const mathSource = " + json.dumps(math_source) + ", indexSource = " + json.dumps(index_source) + ";\n"
        "function run(src, req) { const m = {exports:{}}; vm.runInNewContext(src, {module:m, exports:m.exports, require:req}, {timeout:1000}); return m.exports; }\n"
        "const math = run(mathSource, () => { throw new Error('Unexpected math import'); });\n"
        "assert.equal(math.sumEven([1,2,3,4,-2]),4); assert.equal(math.sumEven([]),0);\n"
        "const load = value => run(indexSource, p => { assert.ok(p === './math' || p === './math.js'); return {sumEven:value}; });\n"
        "const entry = load(math.sumEven); assert.equal(entry.sumEven,math.sumEven); assert.equal(entry.squareEvenSum([1,2,3,4,-2]),16); assert.equal(entry.squareEvenSum([]),0);\n"
        "const sentinel = () => 7; const isolated = load(sentinel); assert.equal(isolated.sumEven,sentinel); assert.equal(isolated.squareEvenSum([2]),49);\n"
        "console.log('both generated modules passed');\n")
