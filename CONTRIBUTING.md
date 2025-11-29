# Contributing to Semifold

Contributions are very welcome! Contributions include but are not restricted to Reporting Bugs, Suggesting Enhancements, and Submitting Pull Requests. Follow the steps below to get started.

1. Fork and clone the repository

   ```bash
   git clone https://github.com/noctisynth/semifold.git # replace with your fork
   ```

2. Create a new branch for your contribution

   ```bash
   git checkout -b my-contribution
   ```

3. Make your changes and commit them

   ```bash
   git add .
   git commit -m "feat: add my contribution"
   ```

   The commit message should follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification.

4. Install the pre-commit hooks

   ```bash
   prek install
   ```

   If you have no `prek` installed, you can install it by running `cargo install prek` or following the [installation guide](https://prek.j178.dev/installation/).

5. Push your changes to your fork

   ```bash
   git push
   ```

   You can now open a pull request to the `main` branch of the original repository.
