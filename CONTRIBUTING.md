# Contributing to Monoxide

🎉 Thank you for your interest in contributing! We’re excited to build Monoxide together.

## 🤝 Before You Start

- Please **open an issue first** if you're planning to introduce new features or change existing behavior.
- For small fixes (typos, code comments, test cases), feel free to submit a PR right away.

## ✅ What You Can Help With

- Bug reports or test failures
- Improving documentation and examples
- Implementing unimplemented methods
- Proposing new macros, traits, or helpers
- Making the developer experience more ergonomic

## 🔧 Getting Started

```bash
# Clone the repo
git clone https://github.com/arshia-eskandari/monoxide.git
cd monoxide

# Run tests
cargo nextest run

# Run examples
cargo run --example basic_usage
```

## 🛆 Development Tips

- Run `cargo fmt` to format code before committing.
- Keep PRs focused — aim for one feature or fix per pull request.
- Follow the existing module structure and naming conventions.

## 🔬 Testing

Make sure to add or update tests for your changes.
We use modular test files per method. Run individual tests like:

```bash
cargo nextest run saves_document_without_id_correctly
```

Use `.clear()` in tests to clean up state when needed.

## 📜 Licensing

By contributing, you agree your code will be released under the MIT license.

## 📬 Questions?

Open a [Discussion](https://github.com/arshia-eskandari/monoxide/discussions) or ping us in an issue.

---
