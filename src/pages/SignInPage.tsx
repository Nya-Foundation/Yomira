import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Globe, Loader2 } from "lucide-react";
import { normalizeError, signInWithCookies, signInWithWebview } from "../lib/api";
import { toast } from "../stores/toast";
import { useT } from "../lib/i18n";
import { Button } from "../components/ui/button";
import { Textarea } from "../components/ui/textarea";
import { Separator } from "../components/ui/separator";

// Session cookies placed in the project-root .env (ipb_member_id, ipb_pass_hash,
// igneous) are exposed by Vite via envPrefix and pre-fill the form. Nothing is
// sent until the user clicks "Sign in".
const env = import.meta.env as Record<string, string | undefined>;
const envCookies = import.meta.env.DEV ? [env.IPB_MEMBER_ID && `ipb_member_id=${env.IPB_MEMBER_ID}`,
  env.IPB_PASS_HASH && `ipb_pass_hash=${env.IPB_PASS_HASH}`,
  env.IGNEOUS && `igneous=${env.IGNEOUS}`].filter(Boolean).join("; ") : "";

export function SignInPage() {
  const t = useT();
  const navigate = useNavigate();
  const [webviewLoading, setWebviewLoading] = useState(false);
  const [cookieLoading, setCookieLoading] = useState(false);
  const [cookies, setCookies] = useState(envCookies);
  const [cookieError, setCookieError] = useState<string | null>(null);

  const done = () => {
    toast.success(t("signin.signedIn"), t("signin.welcome"));
    void navigate("/");
  };

  const onWebview = async () => {
    setWebviewLoading(true);
    try {
      await signInWithWebview();
      done();
    } catch (e) {
      const err = normalizeError(e);
      // User closed the sign-in window, not an error worth surfacing.
      if (err.kind !== "cancelled") {
        toast.error(t("signin.failed"), err.message);
      }
    } finally {
      setWebviewLoading(false);
    }
  };

  const onCookies = async () => {
    setCookieLoading(true);
    setCookieError(null);
    // Common paste typo: "ipb_pass_hash-<value>" with a hyphen instead of "=".
    if (/(?:^|;\s*)ipb_pass_hash-/.test(cookies.trim())) {
      setCookieLoading(false);
      setCookieError(t("signin.cookieError"));
      return;
    }
    try {
      await signInWithCookies(cookies);
      done();
    } catch (e) {
      setCookieError(normalizeError(e).message);
    } finally {
      setCookieLoading(false);
    }
  };

  return (
    <div className="flex h-full overflow-y-auto p-4 sm:p-6">
      <div className="m-auto w-full max-w-sm shrink-0 rounded-lg border border-border bg-surface p-6">
        <h2 className="text-title font-semibold tracking-tight">{t("signin.title")}</h2>
        <p className="mt-1 text-small leading-5 text-muted-foreground">
          {t("signin.subtitle")}
        </p>

        <Button className="mt-5 w-full gap-2" onClick={() => void onWebview()} disabled={webviewLoading || cookieLoading}>
          {webviewLoading ? <Loader2 size={15} className="animate-spin" /> : <Globe size={15} strokeWidth={1.75} />}
          {webviewLoading ? t("signin.webviewWaiting") : t("signin.webview")}
        </Button>

        <div className="my-5 flex items-center gap-3">
          <Separator className="flex-1" />
          <span className="text-caption text-faint-foreground">{t("signin.orCookies")}</span>
          <Separator className="flex-1" />
        </div>

        <Textarea
          value={cookies}
          onChange={(e) => setCookies(e.target.value)}
          placeholder="ipb_member_id=1234567; ipb_pass_hash=abcdef0123...; igneous=..."
          aria-label={t("signin.cookiesAria")}
          className="font-mono text-small"
          rows={4}
        />
        <Button
          variant="subtle"
          className="mt-3 w-full"
          onClick={() => void onCookies()}
          disabled={cookieLoading || webviewLoading || !cookies.trim()}
        >
          {cookieLoading && <Loader2 size={15} className="animate-spin" />}
          {t("signin.submit")}
        </Button>
        {cookieError && (
          <p role="alert" className="mt-2 rounded-md border border-danger/25 bg-danger/5 px-3 py-2 text-caption leading-5 text-danger">
            {cookieError}
          </p>
        )}
        {envCookies && !cookieError && (
          <p className="mt-2 text-caption text-faint-foreground">
            {t("signin.preFilledA")}
            <code className="font-mono">.env</code>
            {t("signin.preFilledB")}
          </p>
        )}

        <p className="mt-4 text-caption leading-5 text-faint-foreground">
          {t("signin.helpA")}
          <code className="font-mono">ipb_member_id</code>
          {t("signin.helpAnd")}
          <code className="font-mono">ipb_pass_hash</code>
          {t("signin.helpEnd")}
        </p>
      </div>
    </div>
  );
}
