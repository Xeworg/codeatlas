import React from 'react';
import { useService3 } from '../services/Service13.ts';
import { helper5 } from '../utils/helper.ts';

interface Props { id: string; label: string; }

export const Component013 = ({ id, label }: Props) => {
  const svc = useService3();
  return <div id={id}>{label}</div>;
};
