import type { DevServerProxy, PathRewrite, ProxyRule } from "@utoo/pack-shared";
import type { IncomingMessage, ServerResponse } from "http";
import httpProxy from "http-proxy";
import type { Duplex } from "stream";

/**
 * Apply pathRewrite (http-proxy-middleware style).
 * Object: first matching regex → replace, then stop.
 * Function: (path) => new path.
 */
function applyPathRewrite(path: string, pathRewrite?: PathRewrite): string {
  if (!pathRewrite) return path;

  if (typeof pathRewrite === "function") {
    return pathRewrite(path);
  }

  for (const [pattern, value] of Object.entries(pathRewrite)) {
    const regex = new RegExp(pattern);
    if (regex.test(path)) {
      return path.replace(regex, value);
    }
  }
  return path;
}

function doesContextMatchUrl(context: string, url: string): boolean {
  return (
    (context[0] === "^" && new RegExp(context).test(url)) ||
    url.startsWith(context)
  );
}

function ruleMatchesUrl(rule: ProxyRule, url: string): boolean {
  const contexts = Array.isArray(rule.context) ? rule.context : [rule.context];
  return contexts.some((ctx) => doesContextMatchUrl(ctx, url));
}

// Single shared proxy instance reused for HTTP and WS proxying.
const sharedProxy = httpProxy.createProxyServer({});

type ProxyOptionsForHttpProxy = {
  target: string;
  changeOrigin: boolean;
  secure?: boolean;
};

type PreparedProxy = {
  rewrittenPath: string;
  options: ProxyOptionsForHttpProxy;
};

/**
 * Shared preparation logic for HTTP and WebSocket proxying:
 * - match rule by URL
 * - apply pathRewrite (req.url → new path)
 * - compute http-proxy options
 */
function prepareProxy(
  url: string | undefined,
  rules: DevServerProxy | undefined,
): PreparedProxy | null {
  if (!rules?.length || !url) return null;

  const matched = rules.find((rule) => ruleMatchesUrl(rule, url));

  if (!matched) return null;

  const rewrittenPath = applyPathRewrite(url, matched.pathRewrite);

  const changeOrigin = matched.changeOrigin ?? true;

  return {
    rewrittenPath,
    options: {
      target: matched.target,
      changeOrigin,
      secure: matched.secure,
    },
  };
}

/**
 * Handle a single HTTP request with devServer.proxy rules.
 *
 * Returns:
 * - true  → the request has been proxied and the response lifecycle is owned by http-proxy
 * - false → no matching rule, caller should continue normal handling
 */
export async function handleProxyRequest(
  req: IncomingMessage,
  res: ServerResponse,
  rules: DevServerProxy | undefined,
): Promise<boolean> {
  const prepared = prepareProxy(req.url, rules);
  if (!prepared) return false;

  req.url = prepared.rewrittenPath;

  await new Promise<void>((resolve) => {
    const done = () => resolve();

    res.on("close", done);

    sharedProxy.web(req, res, prepared.options, (err) => {
      if (err) {
        console.error(
          `[Utoo Pack] Proxy error for ${req.method} ${req.url}:`,
          err,
        );
        if (!res.headersSent) {
          res.writeHead(502, { "Content-Type": "text/plain" });
        }
        if (!res.writableEnded) {
          res.end(`Proxy error: ${err.message}`);
        }
        done();
      }
    });
  });

  return true;
}

/**
 * Handle WebSocket upgrade with devServer.proxy rules.
 *
 * Returns:
 * - true  → the upgrade has been proxied
 * - false → no matching rule, caller may fallback (e.g. close the socket)
 */
export async function handleProxyUpgrade(
  req: IncomingMessage,
  socket: Duplex,
  head: Buffer,
  rules: DevServerProxy | undefined,
): Promise<boolean> {
  const prepared = prepareProxy(req.url, rules);
  if (!prepared) return false;

  req.url = prepared.rewrittenPath;

  await new Promise<void>((resolve) => {
    const done = () => resolve();

    socket.on("close", done);

    sharedProxy.ws(req, socket, head, prepared.options, (err) => {
      if (err) {
        console.error(`[Utoo Pack] Proxy WS error for ${req.url}:`, err);
        socket.destroy(err);
        done();
      }
    });
  });

  return true;
}
