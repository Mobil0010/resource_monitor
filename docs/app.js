const body = document.body;
const repoOverride = body.dataset.repository.trim();
const releaseApiUrl = `https://api.github.com/repos/${repoOverride}/releases/latest`;

const translations = {
  ko: {
    pageTitle: "Resource Monitor — 다운로드", description: "Resource Monitor — CPU, 메모리, 디스크와 네트워크 상태를 한눈에 확인하는 데스크톱 앱",
    homeLabel: "Resource Monitor 홈", navLabel: "주요 메뉴", languageLabel: "언어 선택", navFeatures: "기능",
    eyebrow: "실시간 시스템 모니터링", heroLine1: "내 컴퓨터의", heroLine2: "상태를", heroLine3: "한눈에.",
    heroDescription: "CPU, 메모리, 디스크, 프로세스와 네트워크 사용량을 가볍고 빠르게 확인하세요.", supportedLanguages: "지원 언어",
    checkingOs: "운영체제를 확인하는 중...", latestVersion: "최신 버전", chooseInstaller: "설치 파일을 선택해 주세요",
    preparingInstaller: "접속한 운영체제에 맞는 설치 파일을 준비하고 있습니다.", download: "다운로드", otherPlatforms: "다른 운영체제",
    macDetail: "Apple Silicon + Intel Universal", windowsSetup: "Windows 설치 마법사", windowsDetail: "권장 · Windows 10 / 11",
    portableDetail: "설치하지 않고 실행", loadingRelease: "GitHub Releases에서 최신 설치 파일을 불러오고 있습니다.",
    componentsTitle: "Windows 구성요소 안내", componentsSummary: "온도 센서와 게임 성능 측정에 사용되는 구성요소를 확인하세요.",
    pawnioComponentTitle: "PawnIO · 하드웨어 온도 센서", pawnioComponentBody: "CPU와 C 드라이브처럼 Windows가 기본적으로 공개하지 않는 하드웨어 온도를 읽는 데 사용합니다. 처음 실행할 때 별도의 설치 창과 관리자 권한 요청이 나타날 수 있습니다.",
    pawnioComponentNote: "설치하지 않아도 기본 모니터링은 작동하지만 일부 온도가 표시되지 않을 수 있습니다.",
    presentmonComponentTitle: "PresentMon · FPS와 프레임 시간", presentmonComponentBody: "Steam 게임 팝업에서 초당 프레임과 프레임 시간(ms)을 측정할 때 사용합니다. Windows 설치 마법사와 Portable ZIP에 함께 포함되어 별도로 설치할 필요가 없습니다.",
    presentmonComponentNote: "FPS 또는 프레임 시간 표시를 켠 게임이 실행 중일 때만 측정에 사용합니다.",
    featuresTitle: "필요한 정보만, 실시간으로.", feature1Title: "시스템과 온도 모니터링",
    feature1Body: "CPU 전체·코어별 사용률, GPU, 메모리, 디스크 용량과 CPU·GPU·C 드라이브 온도를 한 화면에서 확인하고 1~10초 간격으로 갱신할 수 있습니다.",
    feature2Title: "프로세스 & 네트워크", feature2Body: "실행 중인 프로세스를 검색하고 CPU 점유율 순으로 확인하며, 다운로드·업로드 속도와 최근 사용 기록을 실시간 그래프로 추적합니다.",
    feature3Title: "자유롭게 설정하는 팝업", feature3Body: "표시 항목과 미니 그래프, 모니터, 화면 위치, 크기, 배경 투명도를 따로 설정할 수 있습니다. 팝업은 클릭을 가로채지 않아 뒤의 게임과 앱을 그대로 조작할 수 있습니다.",
    feature4Title: "Steam 게임 연동", feature4Body: "로컬 Steam 라이브러리를 자동 또는 수동으로 찾고 전체·선택 게임을 감지합니다. 공통 또는 게임별 팝업 설정을 적용하고 게임 종료 후 기존 팝업을 자동으로 복원합니다.",
    releasePage: "GitHub에서 설치 파일 보기", checkingRelease: "GitHub Releases에서 최신 설치 파일을 확인하고 있습니다.",
    latestRelease: "최신 릴리스 보기", releaseProvided: "GitHub Releases에서 제공하는 최신 버전입니다.",
    missingAssets: " 일부 설치 파일은 릴리스 페이지에서 확인해 주세요.",
    releaseError: "최신 버전을 확인하지 못했습니다. GitHub 릴리스 페이지에서 설치 파일을 확인해 주세요.",
    releasedOn: (date) => `${date} 릴리스`,
    platform: {
      mac: { status: "macOS가 감지되었습니다", heading: "macOS에 설치", description: "Universal DMG 하나로 Apple Silicon과 Intel Mac을 모두 지원합니다. DMG를 연 뒤 앱을 Applications 폴더로 옮겨 주세요.", primary: "macOS DMG 다운로드" },
      windows: { status: "Windows가 감지되었습니다", heading: "Windows에 설치", description: "일반 설치에는 설치 마법사를 권장합니다. 설치 권한이 없거나 USB에서 실행하려면 Portable ZIP을 선택해 주세요.", primary: "설치 마법사 다운로드" },
      other: { status: "운영체제를 선택해 주세요", heading: "설치 파일 선택", description: "macOS용 DMG 또는 Windows용 설치 마법사와 Portable ZIP 중에서 선택할 수 있습니다.", primary: "최신 릴리스 보기" },
    },
  },
  en: {
    pageTitle: "Resource Monitor — Download", description: "Resource Monitor — a lightweight desktop app for monitoring CPU, memory, disk, processes, and network activity",
    homeLabel: "Resource Monitor home", navLabel: "Main navigation", languageLabel: "Choose language", navFeatures: "Features",
    eyebrow: "REAL-TIME SYSTEM MONITORING", heroLine1: "Your computer's", heroLine2: "status,", heroLine3: "at a glance.",
    heroDescription: "Monitor CPU, memory, disk, processes, and network activity quickly and effortlessly.", supportedLanguages: "App languages",
    checkingOs: "Detecting your operating system...", latestVersion: "Latest version", chooseInstaller: "Choose an installer",
    preparingInstaller: "Preparing the right download for your operating system.", download: "Download", otherPlatforms: "Other platforms",
    macDetail: "Apple Silicon + Intel Universal", windowsSetup: "Windows Setup Wizard", windowsDetail: "Recommended · Windows 10 / 11",
    portableDetail: "Run without installation", loadingRelease: "Loading the latest installer from GitHub Releases.",
    componentsTitle: "Windows components", componentsSummary: "Learn about the components used for temperature sensors and in-game performance metrics.",
    pawnioComponentTitle: "PawnIO · Hardware temperature sensors", pawnioComponentBody: "PawnIO provides access to hardware temperatures that Windows does not normally expose, including CPU and C drive sensors. A separate setup window and administrator prompt may appear on first launch.",
    pawnioComponentNote: "Core monitoring still works without it, but some temperature readings may be unavailable.",
    presentmonComponentTitle: "PresentMon · FPS and frame time", presentmonComponentBody: "PresentMon measures frames per second and frame time in the Steam game overlay. It is bundled with both the Windows Setup Wizard and Portable ZIP, so no separate installation is required.",
    presentmonComponentNote: "It is used only while a game with FPS or frame-time display enabled is running.",
    featuresTitle: "Only what matters, in real time.", feature1Title: "System and temperature monitoring",
    feature1Body: "View total and per-core CPU usage, GPU, memory, disk capacity, and CPU, GPU, and C drive temperatures in one place with a refresh interval from 1 to 10 seconds.",
    feature2Title: "Processes & network", feature2Body: "Search running processes, sort them by CPU usage, and track real-time download and upload speeds with recent activity graphs.",
    feature3Title: "A fully customizable overlay", feature3Body: "Choose metrics, mini graphs, monitor, screen position, size, and background opacity independently. Click-through behavior lets you keep controlling the game or app underneath.",
    feature4Title: "Steam game integration", feature4Body: "Find local Steam libraries automatically or manually and detect all or selected games. Apply shared or per-game overlay profiles, then restore the normal overlay when the game exits.",
    releasePage: "View downloads on GitHub", checkingRelease: "Checking GitHub Releases for the latest installers.",
    latestRelease: "View latest release", releaseProvided: "This is the latest version available from GitHub Releases.",
    missingAssets: " Some installers are only available on the release page.",
    releaseError: "Unable to check the latest version. Open the GitHub release page to view downloads.",
    releasedOn: (date) => `Released ${date}`,
    platform: {
      mac: { status: "macOS detected", heading: "Install on macOS", description: "One Universal DMG supports both Apple Silicon and Intel Macs. Open the DMG, then move the app to Applications.", primary: "Download macOS DMG" },
      windows: { status: "Windows detected", heading: "Install on Windows", description: "The Setup Wizard is recommended. Choose the Portable ZIP if you cannot install apps or want to run it from a USB drive.", primary: "Download Setup Wizard" },
      other: { status: "Choose your operating system", heading: "Choose a download", description: "Select the macOS DMG, Windows Setup Wizard, or Windows Portable ZIP.", primary: "View latest release" },
    },
  },
};

function detectPlatform() {
  const override = new URLSearchParams(location.search).get("platform");
  if (["mac", "windows", "other"].includes(override)) return override;
  const value = `${navigator.userAgentData?.platform || ""} ${navigator.platform || ""} ${navigator.userAgent}`.toLowerCase();
  if (value.includes("win")) return "windows";
  if (value.includes("mac")) return "mac";
  return "other";
}

function detectLanguage() {
  const override = new URLSearchParams(location.search).get("lang");
  if (["ko", "en"].includes(override)) {
    try { localStorage.setItem("resource-monitor-language", override); } catch {}
    return override;
  }
  try {
    const saved = localStorage.getItem("resource-monitor-language");
    if (["ko", "en"].includes(saved)) return saved;
  } catch {}
  const languages = navigator.languages?.length ? navigator.languages : [navigator.language || "en"];
  return languages.some((value) => value.toLowerCase().startsWith("ko")) ? "ko" : "en";
}

const platform = detectPlatform();
let language = detectLanguage();
let latestRelease = null;
const githubUrl = repoOverride ? `https://github.com/${repoOverride}` : "#";
const releasesUrl = `${githubUrl}/releases/latest`;
const primary = document.querySelector("#primary-download");
const primaryText = primary.querySelector("span");
const portable = document.querySelector("#portable-download");
const osMessage = document.querySelector("#os-message");
const installHeading = document.querySelector("#install-heading");
const installDescription = document.querySelector("#install-description");
const versionLabel = document.querySelector("#version-label");
const releaseNote = document.querySelector("#release-note");
const macDownload = document.querySelector("#mac-download");
const windowsDownload = document.querySelector("#windows-download");
const windowsPortableDownload = document.querySelector("#windows-portable-download");

document.querySelector("#year").textContent = new Date().getFullYear();
document.querySelector("#github-link").href = githubUrl;
body.dataset.platform = platform;
portable.hidden = platform !== "windows";

function renderPlatformCopy() {
  const copy = translations[language].platform[platform];
  osMessage.textContent = copy.status;
  installHeading.textContent = copy.heading;
  installDescription.textContent = copy.description;
  primaryText.textContent = latestRelease?.selected ? copy.primary : translations[language].latestRelease;
}

function renderReleaseCopy() {
  if (!latestRelease) return;
  const copy = translations[language];
  versionLabel.textContent = latestRelease.tag || copy.latestVersion;
  if (latestRelease.error) {
    releaseNote.textContent = copy.releaseError;
    primaryText.textContent = copy.releasePage;
    return;
  }
  const published = latestRelease.publishedAt ? new Date(latestRelease.publishedAt) : null;
  releaseNote.textContent = published && !Number.isNaN(published.getTime())
    ? copy.releasedOn(new Intl.DateTimeFormat(language === "ko" ? "ko-KR" : "en-US", { dateStyle: "long" }).format(published))
    : copy.releaseProvided;
  if (latestRelease.missingAssets) releaseNote.textContent += copy.missingAssets;
}

function applyLanguage(nextLanguage, persist = false) {
  language = nextLanguage;
  const copy = translations[language];
  document.documentElement.lang = language;
  document.title = copy.pageTitle;
  const description = document.querySelector('meta[name="description"]');
  if (description) description.setAttribute("content", copy.description);
  document.querySelectorAll("[data-i18n]").forEach((element) => {
    const value = copy[element.dataset.i18n];
    if (typeof value === "string") element.textContent = value;
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach((element) => {
    const value = copy[element.dataset.i18nAriaLabel];
    if (typeof value === "string") element.setAttribute("aria-label", value);
  });
  document.querySelectorAll("[data-language]").forEach((button) => {
    button.setAttribute("aria-pressed", String(button.dataset.language === language));
  });
  if (persist) {
    try { localStorage.setItem("resource-monitor-language", language); } catch {}
  }
  renderPlatformCopy();
  renderReleaseCopy();
}

document.querySelectorAll("[data-language]").forEach((button) => {
  button.addEventListener("click", (event) => {
    event.preventDefault();
    applyLanguage(button.dataset.language, true);
  });
});

const menuButton = document.querySelector("#other-platforms");
const menu = document.querySelector("#platform-menu");
menuButton.addEventListener("click", () => {
  const open = menu.hidden;
  menu.hidden = !open;
  menuButton.setAttribute("aria-expanded", String(open));
});

function useReleasePage() {
  [primary, portable, macDownload, windowsDownload, windowsPortableDownload].forEach((link) => {
    link.href = releasesUrl;
    link.removeAttribute("aria-disabled");
  });
}

async function loadLatestRelease() {
  useReleasePage();
  releaseNote.textContent = translations[language].checkingRelease;
  try {
    const response = await fetch(releaseApiUrl, { headers: { Accept: "application/vnd.github+json" }, signal: AbortSignal.timeout(10000) });
    if (!response.ok) throw new Error("release unavailable");
    const release = await response.json();
    if (release.draft || release.prerelease || !Array.isArray(release.assets)) throw new Error("release invalid");
    const assetUrl = (suffix) => {
      const asset = release.assets.find((item) => item.name?.startsWith("ResourceMonitor-") && item.name.endsWith(suffix)
        && item.browser_download_url?.startsWith(`${githubUrl}/releases/download/`));
      return asset?.browser_download_url;
    };
    const mac = assetUrl("-macOS-Universal.dmg");
    const windows = assetUrl("-Windows-Setup.exe");
    const zip = assetUrl("-Windows-Portable.zip");
    macDownload.href = mac || releasesUrl;
    windowsDownload.href = windows || releasesUrl;
    windowsPortableDownload.href = zip || releasesUrl;
    portable.href = zip || releasesUrl;
    const selected = platform === "mac" ? mac : platform === "windows" ? windows : null;
    primary.href = selected || releasesUrl;
    latestRelease = { selected: Boolean(selected), tag: release.tag_name, publishedAt: release.published_at, missingAssets: !mac || !windows || !zip, error: false };
    renderPlatformCopy();
    renderReleaseCopy();
  } catch {
    useReleasePage();
    latestRelease = { error: true };
    renderPlatformCopy();
    renderReleaseCopy();
  }
}

applyLanguage(language);
loadLatestRelease();
