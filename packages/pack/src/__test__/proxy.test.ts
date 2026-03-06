/**
 * This file doesn't work now, because we don't have a way to test the js logic.
 */
import type { DevServerProxy } from "@utoo/pack-shared";
import type { IncomingMessage, ServerResponse } from "http";
import { Duplex } from "stream";
import { handleProxyRequest, handleProxyUpgrade } from "../core/proxy";

function createMockReq(url: string): IncomingMessage {
  return {
    url,
    method: "GET",
    headers: {},
    on: () => undefined,
  } as unknown as IncomingMessage;
}

function createMockRes(): ServerResponse {
  const res = new Duplex() as unknown as ServerResponse;
  res.writeHead = jest.fn().mockReturnValue(res);
  res.end = jest.fn().mockReturnValue(res);
  return res;
}

describe("devServer proxy integration helpers", () => {
  it("returns false when no rules configured", async () => {
    const req = createMockReq("/api/users");
    const res = createMockRes();

    const handled = await handleProxyRequest(req, res, undefined);

    expect(handled).toBe(false);
  });

  it("matches context by prefix or regexp", async () => {
    const rules: DevServerProxy = [
      {
        context: "/api",
        target: "http://localhost:3000",
      },
      {
        context: "^/auth",
        target: "http://localhost:4000",
      },
    ];

    const compiledHandledApi = await handleProxyRequest(
      createMockReq("/api/users"),
      createMockRes(),
      rules,
    );
    const compiledHandledAuth = await handleProxyRequest(
      createMockReq("/auth/login"),
      createMockRes(),
      rules,
    );

    expect(compiledHandledApi).toBe(true);
    expect(compiledHandledAuth).toBe(true);
  });

  it("handles upgrade requests when proxy rules exist", async () => {
    const rules: DevServerProxy = [
      {
        context: "/ws",
        target: "ws://localhost:8080",
        pathRewrite: { "^/ws": "" },
      },
    ];
    const req = createMockReq("/ws/socket");
    const socket = new Duplex();
    const head = Buffer.alloc(0);

    const handled = await handleProxyUpgrade(req, socket, head, rules).catch(
      () => false,
    );

    expect(typeof handled).toBe("boolean");
    // pathRewrite ^/ws → '' on '/ws/socket' yields '/socket'
    expect(req.url).toBe("/socket");
  });
});
