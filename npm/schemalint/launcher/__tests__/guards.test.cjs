'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const test = require('node:test');

const { assertAllowedUrl, redirectUrl } = require('../download.cjs');
const {
  assertSafeArchivePath,
  findBinaryInDir,
  getSystemPowerShellPath,
  selectExtractedBinary,
} = require('../extract.cjs');

test('redirects remain HTTPS, allowlisted, and bounded', () => {
  assert.doesNotThrow(() => assertAllowedUrl('https://github.com/owner/repo'));
  assert.throws(() => assertAllowedUrl('http://github.com/owner/repo'), /non-HTTPS/);
  assert.throws(() => assertAllowedUrl('https://github.com.evil.test/a'), /disallowed host/);
  assert.throws(() => redirectUrl('https://github.com/a', 5), /Too many redirects/);
});

test('archive paths and extracted binary selection reject traversal and links', () => {
  assert.doesNotThrow(() => assertSafeArchivePath('schemalint/bin/schemalint'));
  assert.throws(() => assertSafeArchivePath('../outside'), /Unsafe archive entry/);
  assert.throws(() => assertSafeArchivePath('/absolute'), /Unsafe archive entry/);
  assert.throws(() => assertSafeArchivePath('C:\\outside'), /Unsafe archive entry/);

  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'schemalint-extract-'));
  const outside = path.join(path.dirname(dir), 'outside-schemalint');
  fs.writeFileSync(outside, 'outside');
  fs.symlinkSync(outside, path.join(dir, 'schemalint'));
  assert.deepEqual(findBinaryInDir(dir, 'schemalint', 3), []);
  assert.throws(() => selectExtractedBinary(dir, 'schemalint'), /found 0/);
});

test('Windows extraction resolves PowerShell from the absolute system directory', () => {
  const executable = getSystemPowerShellPath({ SystemRoot: 'D:\\Windows' });
  assert.equal(
    executable,
    'D:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe'
  );
  assert(path.win32.isAbsolute(executable));
  assert.throws(
    () => getSystemPowerShellPath({ SystemRoot: '.\\Windows' }),
    /must be absolute/
  );
});
