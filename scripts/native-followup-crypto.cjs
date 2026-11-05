'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const [mode, input, output, purpose] = process.argv.slice(2);
const magic = Buffer.from('RDMAC001');
const expected = {
  dmg: '63456bd032ebd1de7d85e401becb092e894a911b0519dee34d260194a2383d3f',
  receipts: 'd4d522ed264613844043308cc93c3d31a5df3d855bed48da61d3f2e4f97eee87',
};
try {
  if (!['encrypt', 'decrypt'].includes(mode) || !input || !output || !['dmg', 'receipts', 'evidence'].includes(purpose)) throw Error('arguments');
  const encodedKey = process.env.MACOS_READINESS_KEY || '';
  const key = Buffer.from(encodedKey, 'base64');
  if (key.length !== 32 || key.toString('base64') !== encodedKey) throw Error('key');
  const aad = Buffer.from(`dedup-macos-v1:${purpose}`);
  let result;
  if (mode === 'encrypt') {
    const iv = crypto.randomBytes(12);
    const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);
    cipher.setAAD(aad);
    const data = Buffer.concat([cipher.update(fs.readFileSync(input)), cipher.final()]);
    result = Buffer.concat([magic, iv, cipher.getAuthTag(), data]);
  } else {
    const data = fs.readFileSync(input);
    if (data.length < 36 || !data.subarray(0, 8).equals(magic)) throw Error('format');
    const decipher = crypto.createDecipheriv('aes-256-gcm', key, data.subarray(8, 20));
    decipher.setAAD(aad);
    decipher.setAuthTag(data.subarray(20, 36));
    result = Buffer.concat([decipher.update(data.subarray(36)), decipher.final()]);
    if (expected[purpose] && crypto.createHash('sha256').update(result).digest('hex') !== expected[purpose]) throw Error('archive hash');
  }
  fs.writeFileSync(output, result, { flag: 'wx', mode: 0o600 });
} catch (_) {
  console.error('Authenticated cryptographic operation failed.');
  process.exitCode = 1;
}
