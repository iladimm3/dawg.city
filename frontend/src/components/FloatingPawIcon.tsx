import { PawPrint } from "lucide-react";

interface FloatingPawIconProps {
  className?: string;
  size?: number;
  rotation?: number;
}

export function FloatingPawIcon({
  className = "",
  size = 24,
  rotation = 15,
}: FloatingPawIconProps) {
  return (
    <PawPrint
      size={size}
      className={`text-outline opacity-30 ${className}`}
      style={{ transform: `rotate(${rotation}deg)` }}
    />
  );
}
