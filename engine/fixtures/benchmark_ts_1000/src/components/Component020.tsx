import React from 'react';
import { useService5 } from '../services/Service20.ts';
import { helper4 } from '../utils/helper.ts';

interface Props { id: string; label: string; }

export const Component020 = ({ id, label }: Props) => {
  const svc = useService5();
  return <div id={id}>{label}</div>;
};
