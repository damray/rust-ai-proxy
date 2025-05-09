# 🧠 Rust AI Proxy with Palo Alto AIRS + Ollama + OpenWebUI

![Build](https://img.shields.io/badge/build-passing-brightgreen)

This project is a secure AI reverse proxy written in Rust. It acts as a security buffer between:

* ✅ The user (via OpenWebUI)
* 🔒 Palo Alto Networks AIRS (AI Runtime Security)
* 🧠 Ollama running local LLMs (e.g. LLaMA3)

It scans prompts and responses for threats in real time before sending them to the model or back to the user.

---

## Features

| Feature                     | Description                                                          |
| --------------------------- | -------------------------------------------------------------------- |
| ✅ Prompt scanning           | Each user prompt is scanned via AIRS before being sent to the model  |
| ✅ Response scanning         | Each LLM response is scanned before being forwarded to OpenWebUI     |
| 🔁 Transparent forwarding   | All other API calls are transparently proxied to Ollama              |
| 🔐 Security-aware responses | Blocks malicious, toxic or sensitive content based on AIRS detection |
| 🐳 Dockerized               | Fully containerized with support for corporate CA certificates       |
| ✅ Log                      | Every Prompt scanned and Response are logged locally for demo and TS |

---

## Tech stack

* `Rust` + `axum` — async web server / router
* `reqwest` — for outgoing API calls to AIRS and Ollama
* `serde` / `serde_json` — for JSON serialization
* `tokio` — async runtime
* `tower_http` — HTTP logging

---

## How it works

```text
[User via OpenWebUI] ➜ [Rust Proxy] ➜ (Prompt scanned by AIRS) ➜ [Ollama] ➜ (Response scanned by AIRS) ➜ [OpenWebUI display]
```

---

## ⚙️ Environment Variables

| Variable            | Required | Description                    |
| ------------------- | -------- | ------------------------------ |
| `PANW_X_PAN_TOKEN`  | ✅        | API token for AIRS             |
| `PANW_PROFILE_ID`   | ✅        | Security profile ID for AIRS   |
| `PANW_PROFILE_NAME` | ✅        | Security profile name for AIRS |

You have to create a `.env` file to define these locally. Please create the file with value :
PANW_X_PAN_TOKEN=[YOUR TOKEN]
PANW_PROFILE_ID=[Retrieve your Profile ID in SCM]
PANW_PROFILE_NAME=[The profile name created by you]

---

## 🐳 Running with Docker (Step-by-step)

### 1. Clone the project

```bash
git clone https://github.com/your-org/rust-ai-proxy.git
cd rust-ai-proxy
```

### 2. Prepare your `.env` file

Create a `.env` file at the root with:

```bash
PANW_X_PAN_TOKEN=your-token-here
PANW_PROFILE_ID=your-profile-id
PANW_PROFILE_NAME=your-profile-name
```

### 3. Add your CA certificate if required

If your network uses TLS inspection, Please change of networks. I didnt have the time for the moment to accept any TLS inspection inside CARGO during the compilation.


### 4. Build the project with Docker

```bash
docker-compose build rust_ai_proxy
```

### 5. Launch the stack

```bash
docker-compose up -d
```

### 6. Open your browser

Access OpenWebUI via:

[http://localhost:8080](http://localhost:8080)

Then open [http://localhost:8080](http://localhost:8080) to access OpenWebUI.

---

## Example AIRS Scan Result (blocked)

```json
{
  "status": "blocked",
  "message": "⛔ The answer has been blocked by AI Runtime Analysis.",
  "reason": "toxic_content",
  "suggestion": "Please modify your question and avoid any toxic_content"
}
```

---

## 📣 TODO

* [ ] Add retry & timeout policy for model and AIRS requests
* [ ] Add unit tests and error tracing
* [ ] Find the correct way to bypass the tls inspection during the cargo build to download prerequisite mod
---

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. Fork the repository
2. Create your branch: `git checkout -b feature/awesome-feature`
3. Commit your changes: `git commit -am 'Add awesome feature'`
4. Push to the branch: `git push origin feature/awesome-feature`
5. Submit a pull request 🙏

Please ensure your code is tested and clean before PR submission.

---

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

## 🤝 Credits
Based on Dam's Brain
Based on Palo Alto Networks AIRS API + Ollama + OpenWebUI stack.
