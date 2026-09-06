# GitHub 배포 안내

GitHub Actions가 빌드와 배포를 자동 실행합니다. 홈페이지는 GitHub Pages, 설치 파일은 GitHub Releases에서 제공합니다. GCP는 사용하지 않습니다.

## 1. 저장소 준비

이 안내는 공개 저장소 `Mobil0010/resource_monitor`를 기준으로 합니다. 공개 저장소의 표준 GitHub Actions 실행기와 GitHub Pages는 무료로 사용할 수 있지만 서비스 사용 제한은 적용됩니다. 비공개 저장소는 요금제에 따라 Pages 이용 가능 여부와 Actions 무료 한도가 다릅니다.

- [GitHub Actions 요금 안내](https://docs.github.com/en/billing/concepts/product-billing/github-actions)
- [GitHub Pages 사용 제한](https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits)

저장소의 Settings → Actions → General에서 Actions 실행이 허용되어 있는지 확인해 주세요. 별도의 개인 액세스 토큰이나 GCP 변수는 필요하지 않으며, 워크플로에서 제공되는 GITHUB_TOKEN을 사용합니다. 조직 정책이 권한을 제한하는 경우 관리자 확인이 필요합니다.

다른 저장소로 배포하는 경우 docs/index.html의 data-repository와 이 문서의 주소를 실제 소유자/저장소 이름으로 변경해 주세요.

## 2. 홈페이지 게시

1. 변경된 파일을 GitHub의 기본 브랜치(main 또는 master)에 커밋하고 푸시해 주세요.
2. Settings → Pages → Build and deployment → Source에서 **GitHub Actions**를 선택해 주세요.
3. Actions → **Deploy site to GitHub Pages** → Run workflow에서 기본 브랜치를 선택하여 실행해 주세요.
4. 완료 후 Pages 설정 또는 실행 결과에 표시되는 주소를 확인해 주세요.

기본 주소는 https://mobil0010.github.io/resource_monitor/ 입니다. 이후 docs 또는 Pages 워크플로 변경을 main/master에 푸시하면 홈페이지가 자동 갱신됩니다. github-pages 환경에 브랜치 제한이 있다면 배포할 브랜치를 허용해 주세요.

공식 설정 안내: https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages

## 3. 설치 파일 게시

1. 변경 사항을 커밋해 주세요.
2. 커밋을 `master` 브랜치에 푸시해 주세요.
3. GitHub Actions가 Cargo.toml과 Cargo.lock의 패치 버전을 1 올립니다.
4. 자동 버전 커밋과 태그를 만들고 macOS·Windows 설치 파일을 게시합니다.

예를 들어 현재 버전이 0.1.0인 경우:

```bash
git add .
git commit -m "변경 내용"
git push origin master
```

푸시가 완료되면 버전은 `0.1.1`이 되고 `v0.1.1` 태그와 Release가 자동 생성됩니다. 자동 버전 커밋은 GitHub Actions를 다시 실행하지 않으므로 배포가 반복되지 않습니다. 로컬 커밋만으로는 실행되지 않으며 GitHub에 푸시해야 합니다.

Actions의 **Build and publish GitHub release**가 다음 파일을 생성합니다.

- ResourceMonitor-0.1.0-macOS-Universal.dmg
- ResourceMonitor-0.1.0-Windows-Setup.exe
- ResourceMonitor-0.1.0-Windows-Portable.zip
- SHA256SUMS.txt

두 운영체제의 빌드가 모두 성공하면 초안 릴리스에 파일을 업로드한 뒤 공개합니다. 이미 공개된 릴리스의 파일은 자동으로 덮어쓰지 않습니다. 수동 실행도 현재 `master` 버전에서 새 패치 버전을 생성하므로 필요한 경우에만 사용해 주세요.

홈페이지는 GitHub API로 최신 정식 릴리스를 조회하므로 새 버전을 게시할 때 홈페이지를 다시 배포할 필요가 없습니다. 최초 릴리스 전, API 제한 또는 네트워크 오류가 발생한 경우에는 릴리스 페이지로 연결합니다. 브라우저에 인증 토큰을 넣지 않습니다.

## 4. 설치 시 보안 경고

현재 macOS 빌드는 ad-hoc 서명이며 Apple 공증은 자동 구성되어 있지 않습니다. Windows 설치 파일도 별도의 코드 서명 인증서를 설정하지 않았습니다. 따라서 macOS Gatekeeper나 Windows SmartScreen 경고가 나타날 수 있습니다. GitHub에 게시하는 것만으로 운영체제의 신뢰 서명이나 공증을 받는 것은 아닙니다.

Windows Smart App Control을 통과하려면 Microsoft 신뢰 루트 프로그램에 포함된 공급자가 발급한 RSA 코드 서명 인증서가 필요합니다. 인증서를 PFX 형식으로 준비한 뒤 저장소의 Settings → Secrets and variables → Actions에 아래 두 값을 등록하면 실행 파일과 설치 마법사를 자동 서명합니다.

- `WINDOWS_SIGNING_CERTIFICATE_BASE64`: PFX 파일을 Base64로 인코딩한 값
- `WINDOWS_SIGNING_CERTIFICATE_PASSWORD`: PFX 암호

인증서가 등록되지 않은 빌드는 계속 정상 생성되지만 Smart App Control에서 차단될 수 있습니다. 자체 서명 인증서만으로는 일반 사용자 컴퓨터의 신뢰를 얻을 수 없습니다.

## 5. 기존 GCP 설정 정리

이 구성으로 전환해도 이미 생성한 GCP 리소스는 자동 삭제되거나 중지되지 않습니다. Cloud Run, Cloud Storage, Artifact Registry 등 사용 중인 리소스와 비용을 확인해 주세요. 필요한 파일을 백업하고 전환 완료를 확인한 뒤 불필요한 리소스를 별도로 정리해야 합니다.

이 저장소에서 사용하던 GCP_PROJECT_ID, GCP_REGION, GCP_DOWNLOAD_BUCKET, GCP_SERVICE_ACCOUNT, GCP_WIF_PROVIDER 변수는 더 이상 필요하지 않습니다. 다른 작업에서 사용하지 않는지 확인한 뒤 삭제할 수 있습니다. GCP 결제 계정이나 프로젝트는 자동 변경하지 않습니다.
