export type WindowRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type WindowMinSize = {
  width: number;
  height: number;
};

export type ViewportSize = {
  width: number;
  height: number;
};

export type ViewportInsets = {
  top?: number;
  right?: number;
  bottom?: number;
  left?: number;
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(value, max));
}

export function fitRectToViewport(
  rect: WindowRect,
  minSize: WindowMinSize,
  viewport: ViewportSize,
  insets: ViewportInsets = {},
): WindowRect {
  const top = Math.max(0, insets.top ?? 0);
  const right = Math.max(0, insets.right ?? 0);
  const bottom = Math.max(0, insets.bottom ?? 0);
  const left = Math.max(0, insets.left ?? 0);
  const availableWidth = Math.max(0, viewport.width - left - right);
  const availableHeight = Math.max(0, viewport.height - top - bottom);
  const width = clamp(rect.width, Math.min(minSize.width, availableWidth), availableWidth);
  const height = clamp(rect.height, Math.min(minSize.height, availableHeight), availableHeight);
  const maxX = left + Math.max(0, availableWidth - width);
  const maxY = top + Math.max(0, availableHeight - height);
  return {
    x: clamp(rect.x, left, maxX),
    y: clamp(rect.y, top, maxY),
    width,
    height,
  };
}
