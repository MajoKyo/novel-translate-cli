# Dùng Spec Kit cho các phase của novel-translate-cli

Từ 22/11/2026, Phase 03–09 dùng Spec Kit để đi từ yêu cầu đến code, test và bàn giao cho **mỗi task hoặc nhóm đã giao**. Phase 02 dùng cùng quy trình cho phần còn mở và cả những góp ý sau khi người dùng thử bản đã merge. Một bộ `spec.md`, `plan.md`, `tasks.md` ứng với đúng một phiếu. PLAN của điều phối và [graph liên phase](PARALLEL_TASKS.json) tiếp tục quyết định task nào được mở.


## Bộ dùng chung và hồ sơ riêng

GitHub chỉ chứa chín skill trong `.agents/skills/speckit-*/`, script/template/workflow và cấu hình dùng chung trong `.specify/`, quy tắc dự án cùng hướng dẫn này. Các bản ghi cài đặt integration, package thực thi và cấu hình riêng từng máy không thuộc bộ công bố.

Điều phối cấp riêng `PLAN.md`, phiếu giao việc, các bản đặc tả/ADR đã ghim và báo cáo cần thiết. Worktree mới không tự nhận được các tài liệu bị ignore. Mỗi task/lượt lưu `spec.md`, `plan.md`, `tasks.md` và review trong `docs/roadmap/evidence/`, giữ `.docs/` và `.specify/feature.json` trên máy. Không thêm các hồ sơ ấy vào Git để sửa liên kết hoặc làm bằng chứng review. Sách, workspace, output và thông tin xác thực cũng giữ riêng. Các tên tệp mẫu như `spec-template.md` là template dùng chung, khác với hồ sơ thực của một task.

Nhận đủ đầu vào riêng trước khi làm task. Khi một hướng dẫn nhắc PLAN hoặc đường dẫn evidence, đó là tài liệu nhận từ điều phối, không phải tệp cần công bố thêm. Hướng dẫn chỉ dùng ví dụ task chung và đường dẫn tương đối; các đường dẫn tuyệt đối do máy đang chạy và phiếu riêng xác định.

Kiểm quy tắc bỏ qua trước khi tạo hồ sơ riêng. `PLAN.md` và `docs/roadmap/evidence/` đã có trong `.gitignore` của dự án; `.specify/.gitignore` giữ con trỏ, cấu hình và package cục bộ. Với `.docs/`, mỗi clone thêm quy tắc vào Git exclude cục bộ nếu chưa có:

~~~powershell
git check-ignore -q -- .docs/kiem-tra
if ($LASTEXITCODE -eq 1) {
    $excludeFile = git rev-parse --path-format=absolute --git-path info/exclude
    if ($LASTEXITCODE -ne 0) { throw 'Không xác định được Git exclude cục bộ.' }
    Add-Content -LiteralPath $excludeFile -Value '/.docs/'
}
git check-ignore -v -- PLAN.md .docs/kiem-tra docs/roadmap/evidence/kiem-tra .specify/feature.json
~~~

<a id="setup"></a>

## 1. Mở đúng project trong Codex

Checkout đã tích hợp có `.agents/skills/speckit-*/SKILL.md` và `.specify/`. Mở **thư mục gốc của worktree được giao** trong Codex Desktop. Có thể dùng Codex CLI bằng cách chạy `codex` trong PowerShell tại cùng thư mục. Trong ô chat, gõ `$speckit-` để chọn skill. Nếu giao diện chưa cập nhật danh sách sau khi cài, mở lại Codex tại đúng worktree. Các skill phục vụ lập trình này nằm riêng với thư viện Markdown dùng để dịch truyện.

Kiểm nhanh trong PowerShell ở gốc worktree:

~~~powershell
Get-Content .specify/init-options.json
Get-ChildItem .agents/skills -Directory -Filter 'speckit-*' | Select-Object -ExpandProperty Name
~~~

Bản tích hợp của dự án dùng `1.4.3`, `integration = codex`, `script = ps`, với chín skill. Khi đã thấy đủ bộ và [quy tắc của novel-translate-cli](../../.specify/memory/constitution.md), có thể sang mục 2.

**Worktree cũ hoặc máy của cộng tác viên.** Nhận bộ công cụ đã duyệt cùng hướng dẫn và constitution của dự án. Worktree tạo từ một commit đã chứa bộ đó dùng ngay các file trong checkout. Nếu worktree đang ở một commit cũ, tiếp nhận ở đầu phiếu kế tiếp. Phiếu ghi rõ worktree, bản công cụ và quyền dành cho file setup. PLAN riêng lấy theo đường dẫn hoặc bản ghim mà điều phối cấp. Git worktree mới chỉ mang các file đã commit.

Nếu cần tái tạo bộ công cụ trên Windows, dưới đây là lệnh dành cho **worktree đã được giao và chưa có Spec Kit**. Chạy trong PowerShell. Máy đã có `pipx` có thể gọi đúng phiên bản trực tiếp:

~~~powershell
git rev-parse --show-toplevel
git branch --show-current
~~~

Đối chiếu thư mục vừa in với worktree của phiếu rồi chuyển về gốc worktree đó. Dừng nếu đã có cấu hình riêng để đối chiếu phiên bản. Ở thư mục mới:

~~~powershell
$phaseRoot = git rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) { throw 'Hãy mở PowerShell ở worktree của phase.' }
Set-Location -LiteralPath $phaseRoot -ErrorAction Stop
if (Test-Path -LiteralPath '.specify') { throw 'Worktree đã có cấu hình Spec Kit.' }
if (Get-ChildItem '.agents/skills' -Directory -Filter 'speckit-*' -ErrorAction SilentlyContinue) {
    throw 'Worktree đã có skill Spec Kit. Kiểm tra bản đang dùng.'
}
pipx run --spec "specify-cli==1.4.3" specify init --here --force --non-interactive --integration codex --script ps
if ($LASTEXITCODE -ne 0) { throw 'Khởi tạo Spec Kit thất bại.' }
~~~

Sau khi vừa khởi tạo, thay **constitution mẫu** bằng `.specify/memory/constitution.md` và nhận hướng dẫn này từ bản công cụ mà điều phối ghim trong phiếu. Constitution dự án tóm tắt các quy tắc trong AGENTS, PLAN và tài liệu review. Trên máy có checkout gốc, có thể dùng lệnh dưới đây ngay trong worktree vừa khởi tạo. Trên máy của cộng tác viên, thay `$checkoutChuan` bằng thư mục họ đã nhận. Hai file cùng đường dẫn, cùng SHA-256 giữa hai checkout là phép đối chiếu. Khi worktree đã có bản tùy chỉnh, so diff với điều phối trước khi tiếp nhận.

~~~powershell
$checkoutChuan = Read-Host 'Đường dẫn checkout chuẩn do điều phối cung cấp'
$files = '.specify/memory/constitution.md', 'docs/roadmap/SPEC_KIT.md'
$mau = (Get-FileHash '.specify/templates/constitution-template.md' -ErrorAction Stop).Hash
foreach ($file in $files) {
    $nguon = Join-Path $checkoutChuan $file
    $hashNguon = (Get-FileHash -LiteralPath $nguon -ErrorAction Stop).Hash
    if (Test-Path -LiteralPath $file) {
        $hienTai = (Get-FileHash -LiteralPath $file -ErrorAction Stop).Hash
        if ($hienTai -eq $hashNguon) { continue }
        if ($file -ne '.specify/memory/constitution.md' -or $hienTai -ne $mau) {
            throw "Đang có bản đã sửa ở $file. Đối chiếu với điều phối."
        }
    }
    [void][IO.Directory]::CreateDirectory((Join-Path (Get-Location).Path (Split-Path -Parent $file)))
    Copy-Item -LiteralPath $nguon -Destination $file -ErrorAction Stop
    if ((Get-FileHash -LiteralPath $file -ErrorAction Stop).Hash -ne $hashNguon) {
        throw "Hai bản khác nhau ở $file"
    }
}
~~~

Khởi tạo thêm bộ công cụ vào worktree mới có thể thay các file do Spec Kit quản lý. Nhánh `feature/…`, commit, push và PR vẫn do quy trình Git của dự án thực hiện. [Bản Spec Kit 1.4.3](https://github.com/github/spec-kit/releases/tag/v1.4.3) · [Tích hợp Codex](https://github.github.com/spec-kit/integrations/codex.html) · [Cài cho dự án đang phát triển](https://github.github.com/spec-kit/how-to/existing-projects.html)

<a id="use"></a>

## 2. Dùng trong một task

**Nơi lưu.** Chọn `docs/roadmap/evidence/phase-XX/speckit/<ID-phiếu>/rNN` trong worktree được giao. Dùng mã task hoặc mã nhóm trong phiếu, thay `rNN` bằng lượt mới. Ví dụ cho P03-T05 là `docs/roadmap/evidence/phase-03/speckit/P03-T05/r01`. Truyền **SPECIFY_FEATURE_DIRECTORY** cùng giá trị đó trong prompt `$speckit-specify`. Spec Kit lưu lựa chọn ở `.specify/feature.json` để plan, tasks và analyze đọc lại. Mỗi chat có worktree riêng. Checkpoint nhiều task có các lượt độc lập trừ khi điều phối giao thành nhóm trước.

| Khi dùng | Skill gõ trong ô chat Codex | Kết quả cần đọc |
|---|---|---|
| Khi khởi tạo quy tắc dự án hoặc khi người dùng duyệt sửa quy tắc | `$speckit-constitution` | Dự án đã có constitution. Các chat phase cùng dùng bản đã ghim. |
| Nhận phiếu | `$speckit-specify`, rồi `$speckit-clarify` nếu còn lựa chọn ảnh hưởng người dùng | `spec.md` ghi vấn đề, ví dụ, kết quả mong muốn và AC có thể kiểm. Câu hỏi đã có đáp án trong spec/ADR thì dẫn lại nguồn. |
| Chuẩn bị triển khai | `$speckit-plan`, rồi `$speckit-checklist` cho phần cần rà | `plan.md` ghi đường đi dữ liệu, module, quyền sửa, lỗi và phép thử. Checklist rà độ rõ của yêu cầu. |
| Chia việc và so độ khớp | `$speckit-tasks` rồi `$speckit-analyze` | `tasks.md` chỉ chứa việc con của phiếu. So từng việc với AC, ca test và owner. Đưa điểm mâu thuẫn về tài liệu có thẩm quyền và ghim lại lượt bị ảnh hưởng. |
| Đủ đầu vào và quyền sửa | `$speckit-implement` | Model A thực hiện việc con đã giao, chạy kiểm thử tương ứng, tự rà và bàn giao candidate cho B. Task tài liệu tạo đúng đầu ra tài liệu. |
| Cần đối chiếu phần còn thiếu sau khi implement | `$speckit-converge` | Skill có thể **nối thêm việc vào tasks.md**. Dùng trước khi đóng băng, hoặc trên lượt mới. Việc ngoài phạm vi gửi điều phối cấp phiếu riêng. |

A có thể gọi các skill theo thứ tự trong một chat. Các bước có thể xem hoặc chạy riêng khi cần. Dừng ở quyết định sản phẩm thực sự còn mở, quyền sửa protected đang thiếu hoặc lúc bàn giao B. Tên “Phase” và số việc nhỏ được sinh trong `tasks.md` chỉ có nghĩa trong phiếu đó. Khi phiên B kiểm, giữ nguyên spec/plan/tasks và code của candidate. B tự kiểm AC và đưa verdict theo [quy trình review](REVIEW_WORKFLOW.md). Sau verdict thực hiện [checkpoint và merge](MERGE_WORKFLOW.md), giao bản chạy cùng ca để người dùng tự thử.

Hai lệnh PowerShell sau kiểm đúng thư mục hoạt động và quy tắc Git. Ví dụ này dùng P03-T05, chạy sau bước specify:

~~~powershell
Get-Content .specify/feature.json
git check-ignore -v -- docs/roadmap/evidence/phase-03/speckit/P03-T05/r01/spec.md .specify/feature.json
~~~

`plan.md` trong thư mục trên là kế hoạch cho **một phiếu**. PLAN.md trung tâm, việc cấp ID/cổng mở, cập nhật status và merge có một người điều phối. Nếu worktree thiếu PLAN, chat đọc đúng bản điều phối cung cấp. Không đưa bản riêng, evidence hay sách của người dùng lên PR. [Quy tắc repo](../../AGENTS.md)

<a id="phase02"></a>

## 3. Phase 02 đang làm hoặc vừa giao để thử

- **Phiếu đang mở.** Đối chiếu trạng thái mới nhất với điều phối. Với phần chưa đóng băng, nhận Spec Kit vào lượt tiếp nhận của task đang làm hoặc task kế tiếp. Nếu B đang kiểm một candidate, giữ nguyên candidate và đầu vào, rồi tiếp nhận tại lượt sửa/kế tiếp. Ghi lại phần đã làm để tiếp tục đúng chỗ.
- **Có góp ý khi dùng thử.** Điều phối đối chiếu hành vi với AC đã duyệt. Nếu cùng phạm vi của task còn mở, tạo lượt sửa kế tiếp trên đúng task. Nếu là yêu cầu mới hoặc chạm phần đã merge, đăng ký việc sửa và checkpoint phù hợp theo sổ hiện hành. Ghi bản/SHA, cách tái hiện, thực tế, mong muốn, trường hợp đạt/trượt và ai tự thử. Khi cần đổi phần protected đã merge, trình diff tối thiểu cùng tests cho Master theo quy tắc hiện hành.
- **Nghiệm thu.** Mỗi góp ý đi qua specify → plan → tasks → analyze → implement, có test hồi quy, review B và bài thử lại cho Master. Khi M2 còn chờ Master, phản hồi còn mở nằm trong cổng ấy. Nếu M2 đã được Master chốt rồi mới có phản hồi, mở việc tiếp nối và liên kết về lần nghiệm thu cũ.

**Prompt cho chat Phase 02 đang chạy** — thay hai dòng trong ngoặc nếu đã có phản hồi cụ thể:

~~~text
Tiếp tục Phase 02 từ phiếu hiện tại. Đọc docs/roadmap/SPEC_KIT.md, AGENTS.md và PLAN do điều phối cấp.
Nếu checkout này thiếu hướng dẫn/skill, đọc bản ở checkout gốc của điều phối và nhận bộ theo mục 1 ở lượt có thể sửa.
Áp dụng Spec Kit cho phần còn mở và mọi phản hồi sau dùng thử. Giữ nguyên đầu vào đang được B kiểm.
Bản và bước tái hiện: [có thể ghi sau khi tự thử].
Thực tế / em muốn nhìn thấy hoặc làm được: [điền lời người dùng].
Đối chiếu AC hiện hành, chọn lượt sửa hay task mới. Gán SPECIFY_FEATURE_DIRECTORY riêng trong evidence
của đúng task/lượt. Tạo spec, plan, tasks và chạy analyze trước khi implement phần được phép.
Bàn giao B độc lập rồi báo Master đúng bản chạy, thao tác và kết quả cần thấy khi thử lại.
~~~

<a id="phase03-09"></a>

## 4. Prompt dùng từ Phase 03 đến Phase 09

Dán vào mỗi chat phase khi điều phối giao hoặc tiếp nhận phiếu. Một ví dụ cụ thể là [P03-T05 nạp Skill](phases/PHASE-03.md#p03-t05). Giao code T05 sau khi T04 đáp ứng dependency. Với câu hỏi “file nguồn đổi sau khi xem trước”, A đối chiếu quyết định về hash/phiên bản để viết ca thử, hoặc xin quyết định nếu hành vi còn bỏ ngỏ.

~~~text
Bạn là model A của Phase [XX]. Áp dụng docs/roadmap/SPEC_KIT.md cho mọi phiếu của phase này.
Nếu worktree thiếu bộ công cụ, dùng hướng dẫn ở checkout gốc của điều phối, mục 1, để nhận đúng bản 1.4.3.
Đọc AGENTS.md, PLAN do điều phối cấp, thẻ task, spec và ADR liên quan. Nhận đúng task/nhóm,
worktree, checkpoint, phiên bản đầu vào và phạm vi sửa. Nếu đã có work đang làm, nối từ mốc đó.
Cho mỗi phiếu, gọi $speckit-specify với SPECIFY_FEATURE_DIRECTORY đặt tại
docs/roadmap/evidence/phase-XX/speckit/<ID-phiếu>/rNN. Gọi clarify khi cần, rồi plan, checklist
phù hợp, tasks và analyze. Nối từng việc với AC/TC gốc và cách người dùng kiểm kết quả.
Khi đủ quyền thì gọi implement trong phạm vi phiếu. Ghim candidate và bàn giao B độc lập.
Thực hiện checkpoint theo AGENTS.md. Báo Master bản công bố và cách tự thử.
~~~

Ví dụ câu đầu tiên sau khi đã nhận P03-T05:

~~~text
$speckit-specify SPECIFY_FEATURE_DIRECTORY=docs/roadmap/evidence/phase-03/speckit/P03-T05/r01
Đọc P03-T05, mục 7.4 của PHASE-03.md và SKILLS_GLOSSARY_SPEC.md. Mô tả luồng nạp Markdown
chung và xem trước Skill riêng. Làm rõ người dùng thấy gì nếu file đổi giữa lúc xem trước và xác nhận.
Gắn mỗi hành vi với P03-AC05 và ca thử. Giữ các yêu cầu chưa chốt ở phần câu hỏi.
~~~
