'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const [mode, input, output, purpose] = process.argv.slice(2);
const magic = Buffer.from('RDMAC001');
const sourceSha = 'b7db7b652a5eae1e8c0d330f4425da486be9bbc0cd98d6b5ae3f2187ffcb3f22';
try {
  if (!['encrypt', 'decrypt'].includes(mode) || !input || !output || !['candidate-v4', 'evidence-v4'].includes(purpose)) throw Error('arguments');
  const encoded = process.env.MACOS_READINESS_KEY || '';
  const key = Buffer.from(encoded, 'base64');
  if (key.length !== 32 || key.toString('base64') !== encoded) throw Error('key');
  const aad = Buffer.from(`dedup-macos-v4:${purpose}`);
  let result;
  if (mode === 'encrypt') {
    const iv = crypto.randomBytes(12);
    const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);
    cipher.setAAD(aad);
    const inputBytes = fs.readFileSync(input);
    if (purpose === 'candidate-v4' && crypto.createHash('sha256').update(inputBytes).digest('hex') !== sourceSha) throw Error('source hash');
    const ciphertext = Buffer.concat([cipher.update(inputBytes), cipher.final()]);
    result = Buffer.concat([magic, iv, cipher.getAuthTag(), ciphertext]);
  } else {
    const data = fs.readFileSync(input);
    if (data.length < 36 || !data.subarray(0, 8).equals(magic)) throw Error('format');
    const decipher = crypto.createDecipheriv('aes-256-gcm', key, data.subarray(8, 20));
    decipher.setAAD(aad);
    decipher.setAuthTag(data.subarray(20, 36));
    result = Buffer.concat([decipher.update(data.subarray(36)), decipher.final()]);
    if (purpose === 'candidate-v4' && crypto.createHash('sha256').update(result).digest('hex') !== sourceSha) throw Error('source hash');
  }
  fs.writeFileSync(output, result, { flag: 'wx', mode: 0o600 });
} catch (_) {
  console.error('Authenticated cryptographic operation failed.');
  process.exitCode = 1;
}
