import { useEffect, useRef, useState } from 'react';
import { Check, ChevronDown } from 'lucide-react';
import styles from './CustomSelect.module.css';

export interface CustomSelectOption<T extends string> {
  value: T;
  label: string;
  description?: string;
  disabled?: boolean;
}

interface CustomSelectProps<T extends string> {
  value: T;
  options: CustomSelectOption<T>[];
  onChange: (value: T) => void;
  ariaLabel: string;
}

export function CustomSelect<T extends string>({ value, options, onChange, ariaLabel }: CustomSelectProps<T>) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const selected = options.find((option) => option.value === value) ?? options[0];

  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', closeOnEscape);
    };
  }, []);

  return (
    <div className={styles.root} ref={rootRef}>
      <button type="button" className={`${styles.trigger} ${open ? styles.open : ''}`}
        aria-label={ariaLabel} aria-haspopup="listbox" aria-expanded={open}
        onClick={() => setOpen((current) => !current)}>
        <span className={styles.triggerText}>
          <strong>{selected.label}</strong>
          {selected.description && <small>{selected.description}</small>}
        </span>
        <ChevronDown size={17} className={styles.chevron} />
      </button>
      {open && (
        <div className={styles.menu} role="listbox" aria-label={ariaLabel}>
          {options.map((option) => (
            <button type="button" role="option" aria-selected={option.value === value}
              key={option.value} className={`${styles.option} ${option.value === value ? styles.selected : ''}`}
              disabled={option.disabled} onClick={() => { onChange(option.value); setOpen(false); }}>
              <span>
                <strong>{option.label}</strong>
                {option.description && <small>{option.description}</small>}
              </span>
              {option.value === value && <Check size={16} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
