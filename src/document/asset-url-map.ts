const MISSING_IMAGE = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" width="480" height="96"><rect width="100%" height="100%" fill="#f4f4f5"/><text x="24" y="56" font-family="Segoe UI, sans-serif" font-size="18" fill="#71717a">图片缺失</text></svg>',
)}`;

export function missingImageUrl(): string {
  return MISSING_IMAGE;
}

export class AssetUrlMap {
  private blobs = new Set<string>();

  create(bytes: Uint8Array, mime: string): string {
    const copy = new Uint8Array(bytes);
    const url = URL.createObjectURL(new Blob([copy], { type: mime }));
    this.blobs.add(url);
    return url;
  }

  revokeAll(): void {
    for (const url of this.blobs) URL.revokeObjectURL(url);
    this.blobs.clear();
  }
}
