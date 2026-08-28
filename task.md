V1 Goal for Arc API Client

Based on the full audit, here's what I recommend:

### Must-Have for V1 (Critical)

| # | Feature | Why |
|---|---------|-----|
| 1 | **Environment variable interpolation** | Environments exist but are never used in requests. `{{baseUrl}}` replacement in URL, headers, body. This is the #1 gap. |
| 2 | **Keyboard shortcuts** | No Cmd+S, Cmd+W, Cmd+T, Cmd+P, Cmd+Enter (send). Unusable without these. |
| 3 | **Multipart form-data / URL-encoded body** | Most APIs need file uploads or form submissions. Currently only raw text/JSON/HTML. |
| 4 | **Cookies tab in response** | Data is already captured in `Response.cookies` but never displayed. |

### Should-Have for V1 (Polish)

| # | Feature | Why |
|---|---------|-----|
| 5 | **cURL import** | Universal standard. Every API client has it. |
| 6 | **Request timeout setting** | Currently hardcoded ~30s. Users need control. |
| 7 | **Password masking in auth** | Bearer token and Basic Auth passwords shown in plain text. |
| 8 | **Auto Content-Type header** | Currently user must manually add Content-Type for each body type. |
| 9 | **Response search/find** | Can't search large JSON responses. |
| 10 | **Welcome screen quick-start** | Currently static text. Should have "Create Workspace" / "Open" buttons. |

### Nice-to-Have (V1.1+)

- OAuth 2.0 / API Key auth
- Collection import/export
- Request history
- Proxy support
- Binary response handling
- Response export

### What to Skip for V1

- WebSocket support
- HTTP/2 priority
- SSL certificate management
- Drag-and-drop
- Auto-update
- Localization

### V1 Definition

**V1 = A usable daily-driver API client** where you can:
1. Create workspaces and organize requests in a file tree
2. Build requests with all HTTP methods, headers, params, body types, and auth
3. Use environment variables in requests (`{{baseUrl}}/api/users`)
4. Send requests and see response body, headers, status, timing
5. Stress test endpoints
6. Use keyboard shortcuts for all common actions
7. Import cURL commands

**V1 is NOT:** OAuth flows, collection sharing, team features, plugins, or advanced debugging.

Want me to prioritize and plan the implementation order for the must-haves?
