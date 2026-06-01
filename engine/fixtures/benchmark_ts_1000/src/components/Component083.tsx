import React from 'react';
import { useService3 } from '../services/Service3.ts';
import { helper3 } from '../utils/helper.ts';

interface Props { id: string; label: string; }

export const Component083 = ({ id, label }: Props) => {
  const svc = useService3();
  return <div id={id}>{label}</div>;
};
