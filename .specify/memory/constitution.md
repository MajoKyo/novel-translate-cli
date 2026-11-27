# novel-translate-cli · Quy tắc dự án cho Spec Kit

**Phiên bản 1.0.0 · 22/11/2026.** Dùng cho Phase 03–09 và phần việc còn mở hoặc phát sinh khi người dùng thử Phase 02. [Hướng dẫn thực hiện](../../docs/roadmap/SPEC_KIT.md)

## Yêu cầu và phạm vi

Yêu cầu hiện hành của người dùng, [AGENTS.md](../../AGENTS.md), đặc tả, ADR và phiếu do điều phối cấp là căn cứ. Mỗi bộ tài liệu Spec Kit tương ứng một task hoặc một nhóm 2–3 task đã đăng ký trong PLAN. Gắn từng yêu cầu và việc nhỏ với mã task, tiêu chí nghiệm thu, ca kiểm thử và checkpoint gốc. Ghi riêng điểm chưa chốt. Chỉ điều phối thay đổi lịch, người sở hữu và các phụ thuộc.

## Kiểm chứng và công bố

Model A làm trên đúng worktree, phiên bản đầu vào và phạm vi đã giao. B review độc lập trên candidate được giữ ổn định theo [quy trình review](../../docs/roadmap/REVIEW_WORKFLOW.md). Kết quả analyze và dấu hoàn thành trong tasks.md chỉ là đầu vào của lượt review. Nghiệm thu dựa trên ca đã thực chạy, các phép thử thủ công bắt buộc và kết luận của đúng người. Áp dụng các quyền sửa phần đã merge, quy tắc README, commit/push/PR, hậu kiểm và dọn scratch của [quy trình merge](../../docs/roadmap/MERGE_WORKFLOW.md).

## Handoff và dữ liệu

Mỗi worktree dùng một thư mục feature trong evidence đã được Git bỏ qua và một con trỏ feature cục bộ. Ghim các bản spec, plan, tasks đã dùng trước khi giao B. Khi một yêu cầu hoặc candidate đang review cần đổi, ghi lượt tiếp theo cùng bằng chứng. Bản cũ tiếp tục truy được. Giữ sách thật, workspace, khóa và kết quả thực thi ở máy theo AGENTS. Chỉ chia sẻ bộ công cụ và tài liệu được phép xuất bản.
