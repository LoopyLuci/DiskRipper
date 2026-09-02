export function Spinner({ size = 16 }: { size?: number }) {
  return (
    <div
      className="animate-spin rounded-full border-2 border-white/30 border-t-white"
      style={{ width: size, height: size }}
    />
  )
}
