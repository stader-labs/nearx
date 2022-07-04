declare var window: unknown | undefined;

export function isBrowser() {
  return typeof window !== 'undefined';
}

export function nameof<T>(name: keyof T) {
  return name;
}
