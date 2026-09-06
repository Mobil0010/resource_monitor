# Resource Monitor

CPU, GPU, 메모리, 디스크, 프로세스와 네트워크 상태를 실시간으로 확인할 수 있는 크로스 플랫폼 데스크톱 애플리케이션입니다. Rust로 작성했으며 macOS와 Windows를 지원합니다.

## 주요 기능

- 전체 CPU 사용률과 논리 코어별 사용률 표시
- GPU 모델, 사용률, 할당 메모리와 최근 60회 그래프
- 최근 60회 CPU 및 메모리 사용률 그래프
- 물리 메모리, 사용 가능한 메모리와 스왑 사용량 표시
- 마운트된 디스크별 전체, 사용 및 여유 공간 표시
- 실행 중인 프로세스를 CPU 사용률순으로 표시
- 프로세스 이름 검색
- 네트워크 인터페이스별 실시간 업로드 및 다운로드 속도 표시
- 운영체제가 제공하는 CPU, GPU, 디스크 및 기타 온도 센서 표시
- 운영체제 설정에 따른 라이트/다크 테마 자동 적용
- 앱에서 라이트/다크 테마 수동 전환
- 영어, 한국어 및 일본어 인터페이스
- 선택한 정보를 보여주는 항상 위 팝업
- 팝업 위치, 투명도와 미니 그래프 설정
- 메인 앱과 데이터를 공유하며 별도 프로세스를 만들지 않는 보조 팝업
- 시스템 부팅 시 자동 실행
- 1~10초 범위의 시스템 정보 새로고침 간격 설정
- 테마, 언어, 팝업 구성과 새로고침 간격 자동 저장
- 백그라운드에서 하루 한 번 GitHub Releases를 확인하고 앱 안에서 설치 파일을 내려받는 업데이트 기능

## 팝업

설정 화면에서 작은 모니터링 팝업을 활성화할 수 있습니다.

팝업에 표시할 수 있는 정보:

- CPU 사용률
- GPU 사용률
- 메모리 사용률
- 디스크 사용률
- 실행 중인 프로세스 수
- 네트워크 업로드 및 다운로드 속도

지원 위치:

- 왼쪽 상단 / 상단 가운데 / 오른쪽 상단
- 왼쪽 하단 / 하단 가운데 / 오른쪽 하단

팝업은 다른 창보다 항상 위에 표시되며 투명도를 30%부터 100%까지 조절할 수 있습니다.
선택한 정보의 최근 사용량 그래프도 한 줄에 두 개씩 표시할 수 있습니다. 디스크 사용률과 프로세스 수는 값만 표시하고 그래프는 만들지 않습니다. 지원되는 센서가 있으면 CPU, GPU 또는 디스크 값 옆에 온도도 표시합니다. 메인 창의 일반 닫기 버튼을 눌러도 팝업은 유지되며, 사이드바의 프로그램 종료 버튼으로 완전히 종료할 수 있습니다.

팝업은 새 프로그램을 실행하는 방식이 아니라 메인 앱과 같은 프로세스 안에서 생성되는 보조 창입니다. CPU, GPU, 메모리 등의 측정 결과와 그래픽 자원을 공유하므로 팝업을 사용할 때 중복 수집으로 인한 부하를 줄였습니다.

## 지원 환경

| 운영체제 | 상태 | UI 스타일 |
| --- | --- | --- |
| macOS | 지원 | 둥근 카드 중심의 macOS 스타일 |
| Windows 10/11 | 지원 | 각진 형태의 Windows 스타일 |

시스템 정보와 설치된 글꼴을 사용하므로 일부 표시 내용은 운영체제에 따라 달라질 수 있습니다.

## 설치 준비

[Rust 공식 설치 페이지](https://rustup.rs/)에서 Rust를 설치해 주세요. 설치 후 아래 명령으로 확인할 수 있습니다.

```bash
rustc --version
cargo --version
```

이 프로젝트는 Rust 2024 Edition을 사용합니다. 최신 stable Rust 사용을 권장합니다.

## 실행

저장소를 복제하고 프로젝트 폴더에서 실행해 주세요.

```bash
git clone <repository-url>
cd resource_monitor
cargo run
```

최초 실행 시 Rust 의존성과 그래픽 백엔드를 컴파일하므로 시간이 조금 걸릴 수 있습니다. 이후 실행부터는 훨씬 빨라집니다.

## 배포용 빌드

`master` 브랜치에 커밋을 푸시하면 `.github/workflows/release.yml`이 패치 버전을 자동으로 1 올리고 버전 커밋과 태그를 생성합니다. 이어서 아래 설치 파일을 빌드하여 GitHub Releases에 게시합니다. 모든 운영체제의 빌드가 성공한 뒤 공개되며, SHA256SUMS.txt 검증 파일도 함께 제공합니다.

- `ResourceMonitor-0.1.0-macOS-Universal.dmg`
- `ResourceMonitor-0.1.0-Windows-Setup.exe`
- `ResourceMonitor-0.1.0-Windows-Portable.zip`

예를 들어 현재 버전이 `0.1.0`이면 다음 `master` 푸시에서 `0.1.1`로 자동 변경되고 `v0.1.1` Release가 게시됩니다. 로컬 커밋만으로는 실행되지 않으며 GitHub에 푸시해야 합니다.

macOS DMG를 직접 만들려면 macOS에서 다음 명령을 실행해 주세요.

```bash
bash scripts/package-macos.sh 0.1.0
```

Apple Developer 인증서가 없으면 테스트 가능한 ad-hoc 서명 DMG가 생성됩니다. 일반 사용자에게 경고 없이 배포하려면 Developer ID 서명과 Apple 공증이 필요합니다.

Windows 설치 마법사와 Portable ZIP은 Windows GitHub Actions 실행 환경에서 Inno Setup으로 생성됩니다.

## 홈페이지 배포

설치 홈페이지는 GitHub Actions를 통해 GitHub Pages에 배포합니다. 홈페이지는 macOS 접속자에게 Universal DMG를, Windows 접속자에게 설치 마법사와 Portable ZIP을 우선 표시합니다. 설치 파일은 GitHub Releases에서 직접 다운로드합니다.

저장소의 **Settings → Pages → Build and deployment → Source**를 **GitHub Actions**로 설정해 주세요. 이후 main 또는 master 브랜치의 홈페이지 변경 사항이 자동 배포됩니다.

- 홈페이지: https://mobil0010.github.io/resource_monitor/
- 설치 파일: https://github.com/Mobil0010/resource_monitor/releases/latest

위 홈페이지 주소는 Pages 설정과 첫 배포가 완료된 뒤 사용할 수 있습니다. 자세한 초기 설정과 버전 배포 절차는 [GITHUB_DEPLOY.md](./GITHUB_DEPLOY.md)를 확인해 주세요. GCP 계정이나 인증 키는 필요하지 않습니다.

## 사용법

1. 왼쪽 사이드바에서 확인할 항목을 선택해 주세요.
2. `Overview`에서 주요 시스템 상태를 한 번에 확인할 수 있습니다.
3. `Processes`에서 프로세스 이름을 검색하거나 CPU 사용량을 비교할 수 있습니다.
4. `Settings`에서 언어, 팝업 항목, 미니 그래프, 위치, 투명도와 자동 실행 여부를 설정해 주세요.
5. 오른쪽 위 버튼으로 라이트/다크 테마를 전환할 수 있습니다.
6. 메인 창과 팝업을 모두 닫으려면 사이드바의 프로그램 종료 버튼을 눌러 주세요.

## 기술 스택

- [Rust](https://www.rust-lang.org/) — 애플리케이션 언어
- [eframe/egui](https://github.com/emilk/egui) — 네이티브 그래픽 인터페이스
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — 시스템 리소스 정보 수집

## 프로젝트 구조

```text
resource_monitor/
├── .github/workflows/
│   ├── pages.yml
│   └── release.yml
├── Cargo.toml
├── Cargo.lock
├── GITHUB_DEPLOY.md
├── packaging/
│   ├── macos/Info.plist
│   └── windows/resource-monitor.iss
├── scripts/package-macos.sh
├── docs/
├── README.md
└── src/
    └── main.rs
```

## 개발 명령어

홈페이지 다운로드 연결 테스트(Node.js 필요):

```bash
node --test tests/site-downloads.test.mjs
```

코드 포맷 확인:

```bash
cargo fmt --check
```

컴파일 검사:

```bash
cargo check
```

테스트:

```bash
cargo test
```

## 참고 사항

- 시스템 정보는 기본 2초마다 갱신되며 설정에서 1~10초 범위로 변경할 수 있습니다.
- 변경한 설정은 즉시 사용자 설정 폴더에 저장되며 다음 실행 때 자동으로 복원됩니다.
- 현재 화면과 팝업에 표시하지 않는 프로세스, GPU, 디스크 및 네트워크 정보는 불필요하게 다시 수집하지 않습니다.
- 게임 중 부하를 더 줄이려면 새로고침 간격을 3~5초로 설정하고 필요한 팝업 항목만 활성화하는 것을 권장합니다.
- 팝업 위치는 현재 모니터의 화면 크기를 기준으로 계산됩니다.
- macOS에서는 Apple Gothic 계열, Windows에서는 맑은 고딕과 Meiryo 계열 시스템 글꼴을 자동으로 사용합니다.
- GPU 온도 정보는 현재 제공하지 않습니다.


## Code signing policy

See [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md).