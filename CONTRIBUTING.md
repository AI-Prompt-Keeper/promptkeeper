# Contributing to PromptKeeper
First off, thank you for considering contributing! PromptKeeper is a security-focused project, and community review is our strongest asset.

Because we handle sensitive API keys and encryption logic, our contribution process is slightly more rigorous than a typical open-source project.

# 🛡️ Security First
If you discover a security vulnerability, do not open a public issue. Please follow our Responsible Disclosure process:

* Email security@promptkeeper.ai with a description of the vulnerability.
* Provide a Proof of Concept (PoC) if possible.
* Give us 48–72 hours to acknowledge and validate the report before further discussion.

# 🗺️ How Can I Contribute?
## 1. Adversarial Review
We highly value "Security PRs" that don't add features but improve our posture:

* Hardening our AES-256-GCM implementation.
* Improving the Proof-of-Work (PoW) challenge for registration.
* Auditing our Go/Rust dependencies for vulnerabilities.

## 2. Provider Integrations
Help us expand the vault to support more LLM providers (e.g., Mistral, Groq, or local Ollama instances).

Constraint: New providers must support the same "transient in-memory" execution model as OpenAI/Gemini.

## 3. CLI UX & SDKs
Improving the prke CLI experience, improving Android/iOS SDKs, or adding new SDKs (e.g. JS or Python).

# 📝 Pull Request Guidelines
* Open an Issue First: For anything other than a typo fix, please open an issue to discuss the design. We want to ensure the "Dual-Key" security model isn't compromised by new features.
* Atomic Commits: Keep PRs small and focused.
* No Plaintext Secrets: Ensure no test keys or personal credentials are included in your code or commit history.
* Sign Your Commits: We require GPG-signed commits to verify the identity of contributors.
* Tests are Mandatory: Any change to the service-layer or crypto packages must include unit tests with >80% coverage.

# 📜 Contributor License Agreement (CLA)
By contributing to PromptKeeper, you agree that your contributions will be licensed under:

* Apache License 2.0 for core engine/CLI components.
* The PromptKeeper Proprietary License for specific service-layer components.

You retain the copyright to your code, but grant us a perpetual, irrevocable license to use and distribute it under these terms.

# 💬 Community
* GitHub Discussions: For architectural "what if" questions.
* Issues: For confirmed bugs and feature requests.