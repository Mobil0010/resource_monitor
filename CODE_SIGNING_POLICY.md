# Code signing policy

Free code signing provided by SignPath.io, certificate by SignPath Foundation.

## Project

Resource Monitor is an open-source system resource monitoring application for
Windows and macOS.

Repository:
https://github.com/Mobil0010/resource_monitor

## Team roles

- Committer and reviewer: Mobil0010
- Approver: Mobil0010

## Privacy

Resource Monitor displays system resource information locally.

The application does not transmit CPU, GPU, memory, disk, process, network,
or temperature information to external systems.

The application connects to the GitHub API once per day to check for available
updates. Update files are downloaded from GitHub Releases only after the user
requests a download.

## Build and signing

Official releases are built using GitHub Actions from this repository.
Windows release files are submitted to SignPath for Authenticode signing.

Only artifacts built from the official repository may be signed.
