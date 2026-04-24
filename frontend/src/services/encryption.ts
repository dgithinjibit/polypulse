export class EncryptionService {
  private static readonly ALGORITHM = 'AES-GCM';
  private static readonly KEY_LENGTH = 256;

  /**
   * Encrypt bet ID for shareable URL
   */
  static async encryptBetId(betId: string, secret: string): Promise<string> {
    const encoder = new TextEncoder();
    const data = encoder.encode(betId);

    // Derive key from secret
    const keyMaterial = await crypto.subtle.importKey(
      'raw',
      encoder.encode(secret) as Uint8Array,
      'PBKDF2',
      false,
      ['deriveBits', 'deriveKey']
    );

    const key = await crypto.subtle.deriveKey(
      {
        name: 'PBKDF2',
        salt: encoder.encode('polypulse-salt') as Uint8Array,
        iterations: 100000,
        hash: 'SHA-256',
      },
      keyMaterial,
      { name: this.ALGORITHM, length: this.KEY_LENGTH },
      false,
      ['encrypt']
    );

    // Generate random IV
    const iv = crypto.getRandomValues(new Uint8Array(12));

    // Encrypt
    const encrypted = await crypto.subtle.encrypt(
      { name: this.ALGORITHM, iv: iv as Uint8Array },
      key,
      data as Uint8Array
    );

    // Combine IV + ciphertext
    const combined = new Uint8Array(iv.length + encrypted.byteLength);
    combined.set(iv, 0);
    combined.set(new Uint8Array(encrypted), iv.length);

    // Encode as URL-safe base64
    return this.toUrlSafeBase64(combined);
  }

  /**
   * Decrypt bet ID from shareable URL
   */
  static async decryptBetId(encrypted: string, secret: string): Promise<string> {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();

    // Decode from URL-safe base64
    const combined = this.fromUrlSafeBase64(encrypted);

    // Split IV and ciphertext
    const iv = combined.slice(0, 12);
    const ciphertext = combined.slice(12);

    // Derive key from secret
    const keyMaterial = await crypto.subtle.importKey(
      'raw',
      encoder.encode(secret) as Uint8Array,
      'PBKDF2',
      false,
      ['deriveBits', 'deriveKey']
    );

    const key = await crypto.subtle.deriveKey(
      {
        name: 'PBKDF2',
        salt: encoder.encode('polypulse-salt') as Uint8Array,
        iterations: 100000,
        hash: 'SHA-256',
      },
      keyMaterial,
      { name: this.ALGORITHM, length: this.KEY_LENGTH },
      false,
      ['decrypt']
    );

    // Decrypt
    const decrypted = await crypto.subtle.decrypt(
      { name: this.ALGORITHM, iv: iv as Uint8Array },
      key,
      ciphertext as Uint8Array
    );

    return decoder.decode(decrypted);
  }

  /**
   * Generate shareable URL
   */
  static async generateShareableUrl(
    betId: string,
    question: string,
    creatorUsername: string
  ): Promise<string> {
    const secret = import.meta.env.VITE_ENCRYPTION_SECRET || 'default_secret';
    const encryptedId = await this.encryptBetId(betId, secret);

    // Create question slug
    const slug = this.createSlug(question);

    // Format: [question-slug]-creator-[username].polypulse.co.ke?bet=[encrypted_id]
    return `${slug}-creator-${creatorUsername}.polypulse.co.ke?bet=${encryptedId}`;
  }

  /**
   * Create URL slug from question
   */
  private static createSlug(question: string): string {
    let slug = question.toLowerCase();

    // Remove question mark
    slug = slug.replace(/\?/g, '');

    // Replace spaces and special chars with hyphens
    slug = slug.replace(/[^a-z0-9]+/g, '-');

    // Remove leading/trailing hyphens
    slug = slug.replace(/^-+|-+$/g, '');

    // Truncate to 50 chars
    if (slug.length > 50) {
      slug = slug.substring(0, 50);
      slug = slug.replace(/-+$/, '');
    }

    return slug;
  }

  /**
   * Convert to URL-safe base64
   */
  private static toUrlSafeBase64(data: Uint8Array): string {
    const base64 = btoa(String.fromCharCode(...data));
    return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  }

  /**
   * Convert from URL-safe base64
   */
  private static fromUrlSafeBase64(str: string): Uint8Array {
    const base64 = str.replace(/-/g, '+').replace(/_/g, '/');
    const padding = '='.repeat((4 - (base64.length % 4)) % 4);
    const decoded = atob(base64 + padding);
    return new Uint8Array(decoded.split('').map((c) => c.charCodeAt(0)));
  }
}
