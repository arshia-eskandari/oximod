# Contributing to OxiMod

🎉 Thank you for your interest in contributing! We’re excited to build OxiMod together.

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
git clone https://github.com/arshia-eskandari/oximod.git
cd oximod

# Run tests
cargo nextest run

# Run examples
cargo run --example basic_usage
```

## 🛠 Branch Naming Conventions

Please use the following format when naming your branches:

```
type/issue-number/short-description
```

**Examples**:
- `fix/42/missing-aggregate-docs`
- `feat/103/implement-index-macro`
- `docs/77/improve-contributing-guide`

This helps us track what each branch is for and associate it easily with related issues.

## 📦 Pull Request Guidelines

- Keep PRs focused — one feature or fix per pull request.
- Reference the issue number in your PR title or description.
- Add relevant tests and docs when introducing changes.
- Run `cargo fmt` to keep things clean.

## 🔬 Testing

Make sure to add or update tests for your changes.  
We use modular test files per method.

Run individual tests like:

```bash
cargo nextest run saves_document_without_id_correctly
```

Use `.clear()` in tests to reset state between runs.

## 📜 Licensing

By contributing, you agree your code will be released under the MIT license.

## 📬 Questions?

Open a [Discussion](https://github.com/arshia-eskandari/oximod/discussions) or ping us in an issue.
