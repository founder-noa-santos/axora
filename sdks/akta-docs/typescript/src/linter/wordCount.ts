/** Word count: whitespace-separated tokens (normative for akta-docs). */
export function countWords(text: string): number {
  const t = text.trim();
  if (!t) return 0;
  return t.split(/\s+/).length;
}
