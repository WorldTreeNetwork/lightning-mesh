import { cn } from "tailwind-variants";

export { cn };

export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
