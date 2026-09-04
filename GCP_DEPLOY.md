# GCP 배포 안내

설치 홈페이지는 Google Cloud Run에 배포하고 DMG, Windows 설치 마법사와 Portable ZIP은 Google Cloud Storage에 보관합니다. GitHub Releases는 사용하지 않으며 홈페이지는 GCP의 `latest.json`을 읽어 최신 설치 파일을 연결합니다.

## 1. 처음 한 번만 준비하기

[Google Cloud CLI](https://cloud.google.com/sdk/docs/install)를 설치하고 로그인해 주세요.

```bash
gcloud auth login
gcloud config set project YOUR_PROJECT_ID
gcloud services enable run.googleapis.com cloudbuild.googleapis.com artifactregistry.googleapis.com iamcredentials.googleapis.com sts.googleapis.com
```

`YOUR_PROJECT_ID`는 실제 GCP 프로젝트 ID로 바꿔 주세요. 기본 배포 리전은 서울 리전인 `asia-northeast3`을 권장합니다.

설치 파일을 보관할 전 세계에서 고유한 버킷 이름을 정한 뒤 버킷을 만들어 주세요.

```bash
gcloud storage buckets create gs://YOUR_DOWNLOAD_BUCKET \
  --location=asia-northeast3 \
  --uniform-bucket-level-access

gcloud storage buckets add-iam-policy-binding gs://YOUR_DOWNLOAD_BUCKET \
  --member=allUsers \
  --role=roles/storage.objectViewer
```

설치 파일은 누구나 내려받아야 하므로 이 버킷의 객체만 공개됩니다. 소스 코드나 인증 정보는 이 버킷에 업로드하지 않습니다. 조직 정책에서 Public Access Prevention을 강제하고 있다면 공개 버킷 대신 외부 Application Load Balancer와 비공개 버킷 구성이 필요합니다.

## 2. 직접 배포하기

프로젝트 최상위 폴더에서 다음 명령을 실행해 주세요.

```bash
gcloud run deploy resource-monitor-site \
  --source . \
  --region asia-northeast3 \
  --set-env-vars DOWNLOAD_BUCKET=YOUR_DOWNLOAD_BUCKET \
  --allow-unauthenticated
```

Cloud Build가 `Dockerfile`을 빌드한 뒤 Cloud Run에 배포합니다. 완료되면 `https://resource-monitor-site-....run.app` 형식의 주소가 표시됩니다.

이후 홈페이지 파일을 수정했을 때 같은 명령을 다시 실행하면 새 버전으로 교체됩니다.

## 3. GitHub Actions 자동 배포

이 저장소의 `.github/workflows/gcp-site.yml`은 `docs` 또는 배포 설정이 main 브랜치에 반영될 때 Cloud Run을 자동 갱신합니다. 장기 서비스 계정 키 대신 Workload Identity Federation을 사용합니다.

GCP에서 배포용 서비스 계정을 만들고 필요한 역할을 부여해 주세요.

```bash
gcloud iam service-accounts create github-deployer \
  --display-name="GitHub Cloud Run deployer"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:github-deployer@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/run.sourceDeveloper"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:github-deployer@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/serviceusage.serviceUsageConsumer"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:github-deployer@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

gcloud storage buckets add-iam-policy-binding gs://YOUR_DOWNLOAD_BUCKET \
  --member="serviceAccount:github-deployer@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/storage.objectAdmin"
```

Cloud Build가 소스에서 컨테이너를 만들 수 있도록 기본 빌드 서비스 계정에도 역할을 부여해 주세요.

```bash
PROJECT_NUMBER=$(gcloud projects describe YOUR_PROJECT_ID --format="value(projectNumber)")

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:${PROJECT_NUMBER}-compute@developer.gserviceaccount.com" \
  --role="roles/run.builder"
```

그다음 GitHub 저장소 `Mobil0010/resource_monitor`만 허용하는 Workload Identity Provider를 만듭니다.

```bash
gcloud iam workload-identity-pools create github-actions \
  --location=global \
  --display-name="GitHub Actions"

gcloud iam workload-identity-pools providers create-oidc github-provider \
  --location=global \
  --workload-identity-pool=github-actions \
  --issuer-uri="https://token.actions.githubusercontent.com" \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
  --attribute-condition="assertion.repository=='Mobil0010/resource_monitor'"

gcloud iam service-accounts add-iam-policy-binding \
  github-deployer@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --role="roles/iam.workloadIdentityUser" \
  --member="principalSet://iam.googleapis.com/projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/github-actions/attribute.repository/Mobil0010/resource_monitor"
```

자세한 보안 설정은 [GCP의 배포 파이프라인용 Workload Identity Federation 안내](https://cloud.google.com/iam/docs/workload-identity-federation-with-deployment-pipelines)를 참고해 주세요.

GitHub 저장소의 `Settings → Secrets and variables → Actions → Variables`에 다음 값을 등록해 주세요.

| 변수 | 값 예시 |
| --- | --- |
| `GCP_PROJECT_ID` | `my-resource-monitor` |
| `GCP_REGION` | `asia-northeast3` |
| `GCP_SERVICE_ACCOUNT` | `github-deployer@my-resource-monitor.iam.gserviceaccount.com` |
| `GCP_WIF_PROVIDER` | `projects/123456789/locations/global/workloadIdentityPools/github/providers/github-provider` |
| `GCP_DOWNLOAD_BUCKET` | `my-resource-monitor-downloads` |

설정 후 GitHub Actions의 `Deploy site to Google Cloud Run`을 수동 실행하여 첫 홈페이지 배포를 확인해 주세요.

## 4. 설치 파일 게시

`v0.1.0` 같은 Git 태그를 푸시하면 `Build and publish packages to GCP` 작업이 실행됩니다.

```bash
git tag v0.1.0
git push origin v0.1.0
```

작업은 운영체제별 설치 파일을 만든 뒤 다음 위치로 직접 업로드합니다.

```text
gs://YOUR_DOWNLOAD_BUCKET/releases/v0.1.0/ResourceMonitor-0.1.0-macOS-Universal.dmg
gs://YOUR_DOWNLOAD_BUCKET/releases/v0.1.0/ResourceMonitor-0.1.0-Windows-Setup.exe
gs://YOUR_DOWNLOAD_BUCKET/releases/v0.1.0/ResourceMonitor-0.1.0-Windows-Portable.zip
gs://YOUR_DOWNLOAD_BUCKET/latest.json
```

홈페이지는 `/downloads/latest.json`을 통해 Cloud Storage의 최신 버전 정보를 읽습니다. 실제 다운로드도 `storage.googleapis.com` 주소로 이루어지므로 GitHub Releases에는 파일이 생성되지 않습니다.

## 5. 사용자 도메인 연결

Cloud Run 기본 주소는 바로 사용할 수 있습니다. 별도 도메인을 연결할 때는 Google이 권장하는 글로벌 외부 Application Load Balancer를 사용하거나 Firebase Hosting을 Cloud Run 앞에 연결할 수 있습니다.

도메인을 연결한 뒤 DNS와 관리형 인증서가 활성화되면 HTTPS가 자동으로 적용됩니다.

## 6. 배포 비용

이 사이트는 정적 파일만 제공하며 Cloud Run은 요청이 없을 때 인스턴스를 0개로 줄일 수 있습니다. 최소 인스턴스를 별도로 지정하지 않으면 소규모 다운로드 사이트의 유휴 비용을 줄일 수 있습니다.
