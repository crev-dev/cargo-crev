#!/usr/bin/env bash
set -eou pipefail

function job_lint() {
  selfci step start "cargo fmt"
  if ! nix build -L .#ci.cargoFmt ; then
    selfci step fail
  fi

  selfci step start "treefmt"
  if ! nix build -L .#ci.fmt ; then
    selfci step fail
  fi
}

function job_cargo() {
  selfci step start "cargo.lock up to date"
  if ! cargo update --workspace --locked -q; then
    selfci step fail
  fi

  selfci step start "build cargo-crev"
  nix build -L .#ci.cargo-crev

  selfci step start "build workspace"
  nix build -L .#ci.workspace

  selfci step start "clippy"
  if ! nix build -L .#ci.clippy ; then
    selfci step fail
  fi

  selfci step start "nextest"
  if ! nix build -L .#ci.tests ; then
    selfci step fail
  fi
}

case "$SELFCI_JOB_NAME" in
  main)
    selfci job start "lint"
    selfci job start "cargo"
    ;;
  cargo)
    job_cargo
    ;;
  lint)
    job_lint
    ;;
  *)
    echo "Unknown job: $SELFCI_JOB_NAME"
    exit 1
esac
