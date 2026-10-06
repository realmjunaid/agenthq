import type { InputHTMLAttributes } from "react";

export type InputProps = InputHTMLAttributes<HTMLInputElement>;

export function Input({ className = "", ...rest }: InputProps) {
  return (
    <input
      className={`field${className ? ` ${className}` : ""}`}
      {...rest}
    />
  );
}

export function Search({
  onSearch,
  onKeyDown,
  ...rest
}: InputProps & { onSearch?: (value: string) => void }) {
  return (
    <Input
      type="search"
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          onSearch?.(e.currentTarget.value);
        }
        onKeyDown?.(e);
      }}
      {...rest}
    />
  );
}
