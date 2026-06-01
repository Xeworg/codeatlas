import React from 'react';
import { useService5 } from '../services/Service5.ts';
import { helper1 } from '../utils/helper.ts';

interface Props { id: string; label: string; }

export const Component025 = ({ id, label }: Props) => {
  const svc = useService5();
  return <div id={id}>{label}</div>;
};
