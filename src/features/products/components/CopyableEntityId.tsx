import { styles } from "../lib/productListPageStyles";

type Props = {
  label: string;
  id: string;
  isCopied: boolean;
  onCopy: (id: string) => void;
};

export function CopyableEntityId({ label, id, isCopied, onCopy }: Props) {
  const displayId = id.length > 18 ? `${id.slice(0, 8)}...${id.slice(-6)}` : id;

  return (
    <div style={styles.copyIdRow} title={id}>
      <span style={styles.copyIdLabel}>{label}</span>
      <span style={styles.copyIdValue}>{displayId}</span>
      <button
        type="button"
        style={styles.copyIdButton}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onCopy(id);
        }}
      >
        {isCopied ? "Copied" : "Copy"}
      </button>
    </div>
  );
}
