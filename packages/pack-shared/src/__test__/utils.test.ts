/**
 * This file doesn't work now, because we don't have a way to test the js logic.
 */
import { DevServerProxy, ProxyOptions, ProxyRule } from "../config";
import { proxyFromObject } from "../utils";

describe("proxyFromObject", () => {
  it("converts string targets to ProxyRule with default changeOrigin", () => {
    const input: Record<string, string | ProxyOptions> = {
      "/api": "http://localhost:3000",
      "/auth": "http://localhost:5000",
    };

    const rules: DevServerProxy = proxyFromObject(input);

    expect(rules).toEqual<ProxyRule[]>([
      {
        context: "/api",
        target: "http://localhost:3000",
        changeOrigin: true,
      },
      {
        context: "/auth",
        target: "http://localhost:5000",
        changeOrigin: true,
      },
    ]);
  });

  it("merges object targets and preserves options", () => {
    const input: Record<string, string | ProxyOptions> = {
      "/api": {
        target: "http://localhost:3000",
        pathRewrite: { "^/api": "" },
        secure: false,
      },
    };

    const rules: DevServerProxy = proxyFromObject(input);

    expect(rules).toEqual<ProxyRule[]>([
      {
        context: "/api",
        target: "http://localhost:3000",
        pathRewrite: { "^/api": "" },
        secure: false,
        changeOrigin: true,
      },
    ]);
  });

  it("skips falsy values", () => {
    const input: Record<string, string | ProxyOptions | undefined> = {
      "/api": undefined,
      "/auth": "http://localhost:5000",
    };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const rules: DevServerProxy = proxyFromObject(input as any);

    expect(rules).toEqual<ProxyRule[]>([
      {
        context: "/auth",
        target: "http://localhost:5000",
        changeOrigin: true,
      },
    ]);
  });
});
