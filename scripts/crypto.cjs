'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');

const [mode, input, output, purpose] = process.argv.slice(2);
const magic = Buffer.from('RDMAC001');
const expectedSourceSha256 = '24314ac69f11a8766727649f0878735eb21cc408af5dbcf975373d246fa71a16';
try {
  if (!['encrypt', 'decrypt'].includes(mode) || !input || !output || !['source', 'overlay', 'evidence'].includes(purpose)) throw Error('arguments');
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
    if (purpose === 'source' && crypto.createHash('sha256').update(result).digest('hex') !== expectedSourceSha256) throw Error('source hash');
    if (purpose === 'overlay') {
      const expected = fs.readFileSync(path.join(__dirname, '../payload/overlay.sha256'), 'ascii').trim();
      if (!/^[0-9a-f]{64}$/.test(expected) || crypto.createHash('sha256').update(result).digest('hex') !== expected) throw Error('overlay hash');
    }
  }
  fs.writeFileSync(output, result, { flag: 'wx', mode: 0o600 });
} catch (_) {
  console.error('Authenticated cryptographic operation failed.');
  process.exitCode = 1;
}
