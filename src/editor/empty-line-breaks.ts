/** Milkdown 用来占位空段落的整行 `<br />`，不是用户写的行内换行。 */
const EMPTY_LINE_BR = /^[ \t]*<br\s*\/?\s*>[ \t]*\r?$/gim;

/** 去掉空段占位 `<br />`，并折叠因此多出来的空行。 */
export function stripEmptyLineBreaks(markdown: string): string {
  return markdown.replace(EMPTY_LINE_BR, "").replace(/\n{3,}/g, "\n\n");
}
