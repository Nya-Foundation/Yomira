import { expect, test } from "bun:test";
import { ApiError, isNotSignedIn, normalizeError } from "../src/lib/api/errors";

test("native error objects preserve the auth discriminator", () => {
  const error = normalizeError({ kind: "notSignedIn", message: "Session expired" });
  expect(isNotSignedIn(error)).toBe(true);
  expect(error.message).toBe("Session expired");
});

test("wrapped native JSON preserves the bandwidth recovery code", () => {
  const error = normalizeError(new Error('IPC rejected: {"kind":"bandwidthExceeded","message":"Quota exhausted"}'));
  expect(error.kind).toBe("bandwidthExceeded");
  expect(error.code).toBe(509);
  expect(error.message).toBe("Quota exhausted");
});

test("offline failures retain their dedicated recovery state", () => {
  expect(normalizeError('{"kind":"pageUnavailable","message":"File removed"}').kind).toBe("pageUnavailable");
});

test("existing API errors keep identity and unknown errors keep their message", () => {
  const error = new ApiError(509, "Limit", "image509");
  expect(normalizeError(error)).toBe(error);
  expect(normalizeError(new Error("Unexpected failure")).message).toBe("Unexpected failure");
});
