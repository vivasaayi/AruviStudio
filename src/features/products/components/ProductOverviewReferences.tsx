import { getReferenceKindLabel } from "../lib/productReferences";
import type { ProductReference } from "../../../lib/types";
import { styles } from "./ProductOverviewDocument.styles";

export function ReferenceList({ references, title }: { references: ProductReference[]; title: string }) {
  if (references.length === 0) {
    return null;
  }

  return (
    <div>
      <div style={styles.sectionTitle}>{title}</div>
      <div style={styles.referenceList}>
        {references.map((reference) => (
          <div key={reference.id} style={styles.referenceCard}>
            <div style={styles.noteHeading}>{getReferenceKindLabel(reference.reference_kind)}</div>
            <h5 style={styles.referenceTitle}>{reference.title}</h5>
            {reference.content ? <div style={{ ...styles.noteText, marginTop: 8 }}>{reference.content}</div> : null}
            {reference.uri ? (
              <a style={styles.referenceUri} href={reference.uri} target="_blank" rel="noreferrer">
                {reference.uri}
              </a>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
}
