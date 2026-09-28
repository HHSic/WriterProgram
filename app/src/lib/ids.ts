// Ids made on this side, in the same alphabet as crates/core/src/store.rs.

const ALPHABET = '0123456789abcdefghjkmnpqrstvwxyz';

export function newId(): string {
  const bytes = new Uint8Array(12);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => ALPHABET[b % ALPHABET.length]).join('');
}
