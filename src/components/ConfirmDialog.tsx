interface ConfirmDialogProps {
  body: string;
  cancelLabel: string;
  confirmLabel: string;
  onCancel: () => void;
  onConfirm: () => void;
  title: string;
}

export function ConfirmDialog({
  body,
  cancelLabel,
  confirmLabel,
  onCancel,
  onConfirm,
  title,
}: ConfirmDialogProps) {
  return (
    <div className="modal-backdrop" onMouseDown={onCancel}>
      <section
        aria-modal="true"
        className="confirm-dialog"
        onMouseDown={(event) => event.stopPropagation()}
        role="alertdialog"
      >
        <h2>{title}</h2>
        <p>{body}</p>
        <footer>
          <button className="text-button" onClick={onCancel} type="button">
            {cancelLabel}
          </button>
          <button className="danger-button" onClick={onConfirm} type="button">
            {confirmLabel}
          </button>
        </footer>
      </section>
    </div>
  );
}
