// Used before signing: the tag, package versions and release notes must agree.
import { readFileSync, appendFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const tag = process.argv[2];
if (!/^v\d+\.\d+\.\d+$/.test(tag ?? '')) throw new Error('Release a stable version tag such as v0.1.2.');
const version = tag.slice(1);
const json = path => JSON.parse(readFileSync(path, 'utf8'));
const config = json('app/src-tauri/tauri.conf.json');
const lock = json('app/package-lock.json');
const versions = {
  'tauri.conf.json': config.version,
  'package.json': json('app/package.json').version,
  'package-lock.json': lock.version,
  'package-lock.json packages': lock.packages[''].version,
  'Cargo.toml': readFileSync('app/src-tauri/Cargo.toml', 'utf8').match(/^version\s*=\s*"([^"]+)"/m)?.[1],
  'Cargo.lock': readFileSync('Cargo.lock', 'utf8').match(/name = "writer-app"\r?\nversion = "([^"]+)"/)?.[1],
};
for (const [name, value] of Object.entries(versions)) {
  if (value !== version) throw new Error(`${name}: expected ${version}, got ${value}.`);
}
if (!config.bundle.createUpdaterArtifacts || !config.plugins.updater.pubkey) throw new Error('Updater signing must be enabled.');
const notes = readFileSync(`docs/releases/${tag}.md`, 'utf8').trim();
if (!notes) throw new Error('Release notes are required.');
if (process.env.GITHUB_OUTPUT) {
  const delimiter = `WRITER_NOTES_${createHash('sha256').update(notes).digest('hex')}`;
  appendFileSync(process.env.GITHUB_OUTPUT, `tag=${tag}\nnotes<<${delimiter}\n${notes}\n${delimiter}\n`);
}
console.log(`Ready to build ${tag}.`);
