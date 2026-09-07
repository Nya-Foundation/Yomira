
import { invokeOrThrow, isTauri, loadMock } from "./transport";

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

export async function checkSignIn(): Promise<boolean> {
  if (!isTauri()) return (await loadMock()).mock.checkSignIn();
  return invokeOrThrow("check_sign_in");
}

export async function signInWithWebview(): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.signInWithWebview();
  return invokeOrThrow("sign_in_with_webview");
}

export async function signInWithCookies(cookies: string): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.signInWithCookies(cookies);
  return invokeOrThrow("sign_in_with_cookies", { cookies });
}

export async function signOut(): Promise<void> {
  if (!isTauri()) return (await loadMock()).mock.signOut();
  return invokeOrThrow("sign_out");
}
