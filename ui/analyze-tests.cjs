const fs = require('fs');
const path = require('path');

const files = ['e2e/app.spec.ts', 'e2e/components.spec.ts', 'e2e/workflows.spec.ts', 'e2e/panels.spec.ts'];

let allTests = [];
let allDescribes = [];

for (const file of files) {
  const content = fs.readFileSync(path.join(__dirname, file), 'utf8');

  // Get describes
  const describes = content.match(/test\.describe\(['"](.+?)['"]/g) || [];
  describes.forEach(d => {
    const m = d.match(/test\.describe\(['"](.+?)['"]/);
    if (m) allDescribes.push(m[1]);
  });

  // Get all test names with their tags
  const testMatches = content.matchAll(/test\(['"](.+?)['"],\s*\{[^}]*(?:tag:\s*['"](@[\w-]+)['"])?[^}]*\)/g);
  for (const m of testMatches) {
    allTests.push({ name: m[1], tag: m[2] || 'none', file: path.basename(file) });
  }
}

console.log('=== Test Suites ===');
const describes = [...new Set(allDescribes)];
describes.forEach(d => console.log('  -', d));

console.log('\n=== Tests by Tag ===');
const tagMap = {};
allTests.forEach(t => {
  if (!tagMap[t.tag]) tagMap[t.tag] = [];
  tagMap[t.tag].push({ name: t.name, file: t.file });
});

Object.entries(tagMap).sort((a,b) => a[0].localeCompare(b[0])).forEach(([tag, tests]) => {
  console.log('\n[' + tag + '] (' + tests.length + ')');
  tests.forEach(t => console.log('  - ' + t.name + ' (' + t.file + ')'));
});

console.log('\n=== Summary ===');
console.log('Total tests:', allTests.length);
