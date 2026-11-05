'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const [mode, input, output, purpose] = process.argv.slice(2);
const magic = Buffer.from('RDMAC001');
const sourceSha = '358b40179d41f83f896550050afcd511aa0034942ee7bdf2a8f19f092584fd81';
try {
  if (!['encrypt', 'decrypt'].includes(mode) || !input || !output || !['candidate-memory', 'evidence-memory'].includes(purpose)) throw Error('arguments');
  const encoded = process.env.MACOS_READINESS_KEY || '';
  const key = Buffer.from(encoded, 'base64');
  if (key.length !== 32 || key.toString('base64') !== encoded) throw Error('key');
  const aad = Buffer.from(`dedup-macos-memory:${purpose}`);
  let result;
  if (mode === 'encrypt') {
    const iv = crypto.randomBytes(12);
    const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);
    cipher.setAAD(aad);
    const inputBytes = fs.readFileSync(input);
    if (purpose === 'candidate-memory' && crypto.createHash('sha256').update(inputBytes).digest('hex') !== sourceSha) throw Error('source hash');
    const ciphertext = Buffer.concat([cipher.update(inputBytes), cipher.final()]);
    result = Buffer.concat([magic, iv, cipher.getAuthTag(), ciphertext]);
  } else {
    const data = fs.readFileSync(input);
    if (data.length < 36 || !data.subarray(0, 8).equals(magic)) throw Error('format');
    const decipher = crypto.createDecipheriv('aes-256-gcm', key, data.subarray(8, 20));
    decipher.setAAD(aad);
    decipher.setAuthTag(data.subarray(20, 36));
    result = Buffer.concat([decipher.update(data.subarray(36)), decipher.final()]);
    if (purpose === 'candidate-memory' && crypto.createHash('sha256').update(result).digest('hex') !== sourceSha) throw Error('source hash');
  }
  fs.writeFileSync(output, result, { flag: 'wx', mode: 0o600 });
} catch (_) {
  console.error('Authenticated cryptographic operation failed.');
  process.exitCode = 1;
}
