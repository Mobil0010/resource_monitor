# Resource Monitor

CPU, 메모리, 디스크, 프로세스와 네트워크 상태를 실시간으로 확인할 수 있는 크로스 플랫폼 데스크톱 애플리케이션이야. Rust로 작성했으며 macOS와 Windows를 지원해.

## 주요 기능

- 전체 CPU 사용률과 논리 코어별 사용률 표시
- GPU 모델, 사용률, 할당 메모리와 최근 60초 그래프
- 최근 60초간 CPU 및 메모리 사용률 그래프
- 물리 메모리, 사용 가능한 메모리와 스왑 사용량 표시
- 마운트된 디스크별 전체, 사용 및 여유 공간 표시
- 실행 중인 프로세스를 CPU 사용률순으로 표시
- 프로세스 이름 검색
- 네트워크 인터페이스별 실시간 업로드 및 다운로드 속도 표시
- 운영체제 설정에 따른 라이트/다크 테마 자동 적용
- 앱에서 라이트/다크 테마 수동 전환
- 영어, 한국어 및 일본어 인터페이스
- 선택한 정보를 보여주는 항상 위 팝업
- 팝업 위치, 투명도와 미니 그래프 설정
- 메인 창을 닫아도 유지되는 독립 팝업
- 시스템 부팅 시 자동 실행

## 팝업

설정 화면에서 작은 모니터링 팝업을 활성화할 수 있어.

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

팝업은 다른 창보다 항상 위에 표시되며 투명도를 30%부터 100%까지 조절할 수 있어.
선택한 정보의 최근 사용량 그래프도 한 줄에 두 개씩 표시할 수 있어. 메인 창의 일반 닫기 버튼을 눌러도 팝업은 유지되며, 사이드바의 프로그램 종료 버튼으로 완전히 종료할 수 있어.

## 지원 환경

| 운영체제 | 상태 | UI 스타일 |
| --- | --- | --- |
| macOS | 지원 | 둥근 카드 중심의 macOS 스타일 |
| Windows 10/11 | 지원 | 각진 형태의 Windows 스타일 |

시스템 정보와 설치된 글꼴을 사용하므로 일부 표시 내용은 운영체제에 따라 달라질 수 있어.

## 설치 준비

[Rust 공식 설치 페이지](https://rustup.rs/)에서 Rust를 설치해. 설치 후 아래 명령으로 확인할 수 있어.

```bash
rustc --version
cargo --version
```

이 프로젝트는 Rust 2024 Edition을 사용해. 최신 stable Rust 사용을 권장해.

## 실행

저장소를 복제하고 프로젝트 폴더에서 실행해.

```bash
git clone <repository-url>
cd resource_monitor
cargo run
```

최초 실행 시 Rust 의존성과 그래픽 백엔드를 컴파일하므로 시간이 조금 걸릴 수 있어. 이후 실행부터는 훨씬 빨라져.

## 배포용 빌드

### macOS

```bash
cargo build --release
./target/release/resource_monitor
```

생성 파일:

```text
target/release/resource_monitor
```

### Windows

PowerShell 또는 명령 프롬프트에서 실행해.

```powershell
cargo build --release
.\target\release\resource_monitor.exe
```

생성 파일:

```text
target\release\resource_monitor.exe
```

macOS와 Windows 실행 파일은 각 운영체제에서 별도로 빌드하는 것을 권장해.

## 사용법

1. 왼쪽 사이드바에서 확인할 항목을 선택해.
2. `Overview`에서 주요 시스템 상태를 한 번에 확인해.
3. `Processes`에서 프로세스 이름을 검색하거나 CPU 사용량을 비교해.
4. `Settings`에서 언어, 팝업 항목, 미니 그래프, 위치, 투명도와 자동 실행 여부를 설정해.
5. 오른쪽 위 버튼으로 라이트/다크 테마를 전환해.
6. 메인 창과 팝업을 모두 닫으려면 사이드바의 프로그램 종료 버튼을 눌러.

## 기술 스택

- [Rust](https://www.rust-lang.org/) — 애플리케이션 언어
- [eframe/egui](https://github.com/emilk/egui) — 네이티브 그래픽 인터페이스
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — 시스템 리소스 정보 수집

## 프로젝트 구조

```text
resource_monitor/
├── Cargo.toml
├── Cargo.lock
├── README.md
└── src/
    └── main.rs
```

## 개발 명령어

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

- 시스템 정보는 1초마다 갱신돼.
- 팝업 위치는 현재 모니터의 화면 크기를 기준으로 계산돼.
- macOS에서는 Apple Gothic 계열, Windows에서는 맑은 고딕과 Meiryo 계열 시스템 글꼴을 자동으로 사용해.
- GPU 온도 정보는 현재 제공하지 않아.
