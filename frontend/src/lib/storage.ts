// localStorage can throw (private mode, blocked site data, quota), so every
// access degrades to "nothing stored" instead of breaking the app.

export function readStorage(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writeStorage(key: string, value: string | null): void {
  try {
    if (value === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, value);
  } catch {
    // Persisting is best effort; in-memory state still works for this session.
  }
}
